use std::collections::HashMap;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    documents: RwLock<HashMap<Url, String>>,
}

impl Backend {
    async fn analyze(&self, uri: Url, text: &str) {
        let diagnostics = match sceneasm_core::assemble(text, sceneasm_core::Target::c64()) {
            Ok(assembly) => assembly
                .diagnostics
                .iter()
                .filter_map(|diagnostic| {
                    let primary = diagnostic.primary.as_ref()?;
                    let span = &primary.span;
                    let severity = match diagnostic.severity {
                        sceneasm_core::diagnostic::Severity::Error => DiagnosticSeverity::ERROR,
                        sceneasm_core::diagnostic::Severity::Warning => DiagnosticSeverity::WARNING,
                        sceneasm_core::diagnostic::Severity::Info => {
                            DiagnosticSeverity::INFORMATION
                        }
                        sceneasm_core::diagnostic::Severity::Hint => DiagnosticSeverity::HINT,
                    };
                    Some(Diagnostic {
                        range: Range {
                            start: Position::new(
                                span.line.saturating_sub(1) as u32,
                                span.column_start.saturating_sub(1) as u32,
                            ),
                            end: Position::new(
                                span.line.saturating_sub(1) as u32,
                                span.column_end.saturating_sub(1) as u32,
                            ),
                        },
                        severity: Some(severity),
                        code: Some(NumberOrString::String(diagnostic.code.into())),
                        source: Some("sceneasm".into()),
                        message: diagnostic.message.clone(),
                        ..Diagnostic::default()
                    })
                })
                .collect(),
            Err(error) => vec![Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("sceneasm".into()),
                message: error.to_string(),
                ..Diagnostic::default()
            }],
        };
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                document_symbol_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions::default()),
                inlay_hint_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "SceneASM".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "SceneASM LSP ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let text = {
            let documents = self.documents.read().await;
            let Some(text) = documents.get(&uri) else {
                return Ok(None);
            };
            text.clone()
        };
        let Ok(assembly) = sceneasm_core::assemble(&text, sceneasm_core::Target::c64()) else {
            return Ok(None);
        };
        let line_no = position.line as usize + 1;
        let column = position.character as usize + 1;

        if let Some(instruction) = assembly.instructions.iter().find(|instruction| {
            instruction.source.line == line_no
                && column >= instruction.source.column_start
                && column <= instruction.source.column_end
        }) {
            let extra = match instruction.opcode.extra_cycle {
                sceneasm_core::ExtraCycle::None => 0,
                sceneasm_core::ExtraCycle::PageCross | sceneasm_core::ExtraCycle::BranchTaken => 1,
                sceneasm_core::ExtraCycle::BranchTakenAndPageCross => 2,
            };
            let base = instruction.opcode.cycles as u16;
            let worst = base + extra;
            let mut value = format!(
                "**{}**  \\nAddress: `${:04x}`  \\nMode: `{:?}`  \\nCycles: **{}**{}",
                instruction.opcode.mnemonic,
                instruction.address,
                instruction.opcode.mode,
                base,
                if worst == base {
                    String::new()
                } else {
                    format!("–{}", worst)
                }
            );
            if let Some(write) = assembly.hardware_writes.iter().find(|write| {
                write.source.line == instruction.source.line
                    && write.source.column_start == instruction.source.column_start
            }) {
                value.push_str(&format!(
                    "\\n\\n**Hardware write:** `{}` (`${:04x}`) = `#${:02x}`",
                    write.register.name, write.register.address, write.value
                ));
                let decoded: Vec<_> =
                    sceneasm_core::c64_registers::decode_value(write.register.address, write.value)
                        .collect();
                if !decoded.is_empty() {
                    value.push_str("\\n\\n**Decoded value**");
                    for field in decoded {
                        value.push_str(&format!(
                            "\\n- `{}` **{}** = `{}` — {}",
                            field.field.bits,
                            field.field.name,
                            field.value,
                            field.field.description
                        ));
                    }
                }
            }
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: None,
            }));
        }

        let line = text.lines().nth(position.line as usize).unwrap_or("");
        let byte = position.character as usize;
        let is_symbol = |ch: char| ch == '_' || ch.is_ascii_alphanumeric();
        let start = line[..byte.min(line.len())]
            .rfind(|ch: char| !is_symbol(ch))
            .map_or(0, |i| i + 1);
        let end = line[byte.min(line.len())..]
            .find(|ch: char| !is_symbol(ch))
            .map_or(line.len(), |i| byte.min(line.len()) + i);
        let name = &line[start..end];
        if let Some(register) = sceneasm_core::c64_registers::register_by_name(name) {
            let mut value = format!(
                "**{}** — C64 hardware register  \\nAddress: `${:04x}`  \\n{}",
                register.name, register.address, register.description
            );
            let fields: Vec<_> =
                sceneasm_core::c64_registers::bit_fields(register.address).collect();
            if !fields.is_empty() {
                value.push_str("\\n\\n**Bits**");
                for field in fields {
                    value.push_str(&format!(
                        "\\n- `{}` **{}** — {}",
                        field.bits, field.name, field.description
                    ));
                }
            }
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(Range {
                    start: Position::new(position.line, start as u32),
                    end: Position::new(position.line, end as u32),
                }),
            }));
        }

        let address_token = {
            let bytes = line.as_bytes();
            let cursor = byte.min(bytes.len());
            let mut token_start = cursor;
            while token_start > 0 && (bytes[token_start - 1] as char).is_ascii_hexdigit() {
                token_start -= 1;
            }
            if token_start > 0 && bytes[token_start - 1] == b'$' {
                token_start -= 1;
            }
            let mut token_end = cursor;
            while token_end < bytes.len() && (bytes[token_end] as char).is_ascii_hexdigit() {
                token_end += 1;
            }
            &line[token_start..token_end]
        };
        if let Some(hex) = address_token.strip_prefix('$') {
            if let Ok(address) = u16::from_str_radix(hex, 16) {
                if let Some(register) = sceneasm_core::c64_registers::register_by_address(address) {
                    let mut value = format!(
                        "**{}** — C64 hardware register  \\nAddress: `${:04x}`  \\n{}",
                        register.name, register.address, register.description
                    );
                    let fields: Vec<_> =
                        sceneasm_core::c64_registers::bit_fields(register.address).collect();
                    if !fields.is_empty() {
                        value.push_str("\\n\\n**Bits**");
                        for field in fields {
                            value.push_str(&format!(
                                "\\n- `{}` **{}** — {}",
                                field.bits, field.name, field.description
                            ));
                        }
                    }
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value,
                        }),
                        range: None,
                    }));
                }
            }
        }

        let Some(value) = assembly.symbols.get(name) else {
            return Ok(None);
        };
        let Some(span) = assembly.symbol_definitions.get(name) else {
            return Ok(None);
        };
        let kind = if text
            .lines()
            .nth(span.line - 1)
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .ends_with(':')
        {
            "Label"
        } else {
            "Constant"
        };
        let hover_value = format!(
            "**{}** — {}  \\nValue: `${:04x}` / {}  \\nDefined: line {}",
            name, kind, value, value, span.line
        );
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_value,
            }),
            range: Some(Range {
                start: Position::new(position.line, start as u32),
                end: Position::new(position.line, end as u32),
            }),
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let text = {
            let documents = self.documents.read().await;
            let Some(text) = documents.get(&uri) else {
                return Ok(None);
            };
            text.clone()
        };
        let line = text.lines().nth(position.line as usize).unwrap_or("");
        let byte = position.character as usize;
        let is_symbol = |ch: char| ch == '_' || ch.is_ascii_alphanumeric();
        let start = line[..byte.min(line.len())]
            .rfind(|ch: char| !is_symbol(ch))
            .map_or(0, |i| i + 1);
        let end = line[byte.min(line.len())..]
            .find(|ch: char| !is_symbol(ch))
            .map_or(line.len(), |i| byte.min(line.len()) + i);
        let name = &line[start..end];
        let Ok(assembly) = sceneasm_core::assemble(&text, sceneasm_core::Target::c64()) else {
            return Ok(None);
        };
        let Some(span) = assembly.symbol_definitions.get(name) else {
            return Ok(None);
        };
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: Range {
                start: Position::new((span.line - 1) as u32, (span.column_start - 1) as u32),
                end: Position::new((span.line - 1) as u32, (span.column_end - 1) as u32),
            },
        })))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let text = {
            let documents = self.documents.read().await;
            let Some(text) = documents.get(&uri) else {
                return Ok(None);
            };
            text.clone()
        };
        let Ok(assembly) = sceneasm_core::assemble(&text, sceneasm_core::Target::c64()) else {
            return Ok(None);
        };
        let mut hints = Vec::new();
        for instruction in &assembly.instructions {
            let line_index = instruction.source.line.saturating_sub(1) as u32;
            if line_index < params.range.start.line || line_index > params.range.end.line {
                continue;
            }
            let extra = match instruction.opcode.extra_cycle {
                sceneasm_core::ExtraCycle::None => 0,
                sceneasm_core::ExtraCycle::PageCross | sceneasm_core::ExtraCycle::BranchTaken => 1,
                sceneasm_core::ExtraCycle::BranchTakenAndPageCross => 2,
            };
            let base = instruction.opcode.cycles as u16;
            let worst = base + extra;
            let mut label = if worst == base {
                format!("{}c", base)
            } else {
                format!("{}–{}c", base, worst)
            };
            let raster_slot = assembly.raster_contracts.iter().find_map(|contract| {
                contract
                    .schedule
                    .iter()
                    .find(|scheduled| {
                        scheduled.address == instruction.address
                            && scheduled.source.line == instruction.source.line
                    })
                    .map(|scheduled| (contract.line, scheduled))
            });
            let timing_tooltip = if let Some((raster_line, scheduled)) = raster_slot {
                label.push_str(&format!(" · r{} c{}", raster_line, scheduled.start_cycle));
                format!(
                    "{} {:?} timing; raster {} cycle {}–{}, {} VIC-II stall cycle(s)",
                    instruction.opcode.mnemonic,
                    instruction.opcode.mode,
                    raster_line,
                    scheduled.start_cycle,
                    scheduled.end_cycle,
                    scheduled.stalled_cycles
                )
            } else {
                format!(
                    "{} {:?} timing",
                    instruction.opcode.mnemonic, instruction.opcode.mode
                )
            };
            hints.push(InlayHint {
                position: Position::new(
                    line_index,
                    instruction.source.column_end.saturating_sub(1) as u32,
                ),
                label: InlayHintLabel::String(label),
                kind: Some(InlayHintKind::PARAMETER),
                text_edits: None,
                tooltip: Some(InlayHintTooltip::String(timing_tooltip)),
                padding_left: Some(true),
                padding_right: Some(false),
                data: None,
            });
        }
        for write in &assembly.hardware_writes {
            let line_index = write.source.line.saturating_sub(1) as u32;
            if line_index < params.range.start.line || line_index > params.range.end.line {
                continue;
            }
            let source_line = text.lines().nth(line_index as usize).unwrap_or("");
            hints.push(InlayHint {
                position: Position::new(line_index, source_line.len() as u32),
                label: InlayHintLabel::String(format!(
                    "  {} = ${:02x}",
                    write.register.name, write.value
                )),
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: Some(InlayHintTooltip::String(write.register.description.into())),
                padding_left: Some(true),
                padding_right: Some(false),
                data: None,
            });
        }
        Ok(Some(hints))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let text = {
            let documents = self.documents.read().await;
            let Some(text) = documents.get(&uri) else {
                return Ok(None);
            };
            text.clone()
        };
        let mut items = Vec::new();
        let mut mnemonics = std::collections::BTreeSet::new();
        for opcode in sceneasm_core::OPCODES {
            if mnemonics.insert(opcode.mnemonic) {
                items.push(CompletionItem {
                    label: opcode.mnemonic.to_ascii_lowercase(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("MOS 6502/6510 instruction".into()),
                    ..CompletionItem::default()
                });
            }
        }
        for directive in [
            ".org",
            ".byte",
            ".text",
            ".assert",
            ".assert_cycles",
            ".raster",
            ".undocumented",
            ".vic_display",
            ".vic_yscroll",
            ".vic_sprites",
            ".vic_sprite_y",
        ] {
            items.push(CompletionItem {
                label: directive.into(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("SceneASM directive".into()),
                ..CompletionItem::default()
            });
        }
        for register in sceneasm_core::c64_registers::registers() {
            items.push(CompletionItem {
                label: register.name.into(),
                kind: Some(CompletionItemKind::VALUE),
                detail: Some(format!(
                    "${:04x} — {}",
                    register.address, register.description
                )),
                insert_text: Some(format!("${:04x}", register.address)),
                ..CompletionItem::default()
            });
        }
        if let Ok(assembly) = sceneasm_core::assemble(&text, sceneasm_core::Target::c64()) {
            for (name, value) in &assembly.symbols {
                let is_label = assembly.symbol_definitions.get(name).is_some_and(|span| {
                    text.lines()
                        .nth(span.line - 1)
                        .unwrap_or("")
                        .split(';')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .ends_with(':')
                });
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(if is_label {
                        CompletionItemKind::REFERENCE
                    } else {
                        CompletionItemKind::CONSTANT
                    }),
                    detail: Some(format!("${:04x}", value)),
                    ..CompletionItem::default()
                });
            }
        }
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let text = {
            let documents = self.documents.read().await;
            let Some(text) = documents.get(&uri) else {
                return Ok(None);
            };
            text.clone()
        };
        let Ok(assembly) = sceneasm_core::assemble(&text, sceneasm_core::Target::c64()) else {
            return Ok(None);
        };
        let mut symbols = Vec::new();
        for (name, span) in &assembly.symbol_definitions {
            let source_line = text.lines().nth(span.line - 1).unwrap_or("");
            let code = source_line.split(';').next().unwrap_or("").trim();
            let is_label = code.ends_with(':');
            let range = Range {
                start: Position::new((span.line - 1) as u32, (span.column_start - 1) as u32),
                end: Position::new((span.line - 1) as u32, (span.column_end - 1) as u32),
            };
            symbols.push(DocumentSymbol {
                name: name.clone(),
                detail: assembly
                    .symbols
                    .get(name)
                    .map(|value| format!("${:04x}", value)),
                kind: if is_label {
                    SymbolKind::FUNCTION
                } else {
                    SymbolKind::CONSTANT
                },
                tags: None,
                deprecated: None,
                range,
                selection_range: range,
                children: None,
            });
        }
        symbols.sort_by_key(|symbol| (symbol.range.start.line, symbol.range.start.character));
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;
        let text = {
            let documents = self.documents.read().await;
            let Some(text) = documents.get(&uri) else {
                return Ok(None);
            };
            text.clone()
        };
        let line = text.lines().nth(position.line as usize).unwrap_or("");
        let byte = position.character as usize;
        let is_symbol = |ch: char| ch == '_' || ch.is_ascii_alphanumeric();
        let start = line[..byte.min(line.len())]
            .rfind(|ch: char| !is_symbol(ch))
            .map_or(0, |i| i + 1);
        let end = line[byte.min(line.len())..]
            .find(|ch: char| !is_symbol(ch))
            .map_or(line.len(), |i| byte.min(line.len()) + i);
        let name = &line[start..end];
        let Ok(assembly) = sceneasm_core::assemble(&text, sceneasm_core::Target::c64()) else {
            return Ok(None);
        };
        if !assembly.symbol_definitions.contains_key(name) {
            return Ok(None);
        }
        Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: Range {
                start: Position::new(position.line, start as u32),
                end: Position::new(position.line, end as u32),
            },
            placeholder: name.to_string(),
        }))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let text = {
            let documents = self.documents.read().await;
            let Some(text) = documents.get(&uri) else {
                return Ok(None);
            };
            text.clone()
        };
        let line = text.lines().nth(position.line as usize).unwrap_or("");
        let byte = position.character as usize;
        let is_symbol = |ch: char| ch == '_' || ch.is_ascii_alphanumeric();
        let start = line[..byte.min(line.len())]
            .rfind(|ch: char| !is_symbol(ch))
            .map_or(0, |i| i + 1);
        let end = line[byte.min(line.len())..]
            .find(|ch: char| !is_symbol(ch))
            .map_or(line.len(), |i| byte.min(line.len()) + i);
        let name = &line[start..end];
        let valid_new_name = {
            let mut chars = params.new_name.chars();
            chars
                .next()
                .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
                && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        };
        if !valid_new_name {
            return Ok(None);
        }
        let Ok(assembly) = sceneasm_core::assemble(&text, sceneasm_core::Target::c64()) else {
            return Ok(None);
        };
        if !assembly.symbol_definitions.contains_key(name)
            || assembly.symbols.contains_key(&params.new_name)
        {
            return Ok(None);
        }

        let mut edits = Vec::new();
        for (line_index, source_line) in text.lines().enumerate() {
            let code = source_line.split(';').next().unwrap_or("");
            let bytes = code.as_bytes();
            let mut index = 0;
            while index < bytes.len() {
                let ch = bytes[index] as char;
                if ch == '_' || ch.is_ascii_alphabetic() {
                    let token_start = index;
                    index += 1;
                    while index < bytes.len() {
                        let ch = bytes[index] as char;
                        if ch == '_' || ch.is_ascii_alphanumeric() {
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    if &code[token_start..index] == name {
                        edits.push(TextEdit {
                            range: Range {
                                start: Position::new(line_index as u32, token_start as u32),
                                end: Position::new(line_index as u32, index as u32),
                            },
                            new_text: params.new_name.clone(),
                        });
                    }
                } else {
                    index += 1;
                }
            }
        }
        let mut changes = HashMap::new();
        changes.insert(uri, edits);
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..WorkspaceEdit::default()
        }))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri;
        let text = {
            let documents = self.documents.read().await;
            let Some(text) = documents.get(&uri) else {
                return Ok(None);
            };
            text.clone()
        };
        let line = text.lines().nth(position.line as usize).unwrap_or("");
        let byte = position.character as usize;
        let is_symbol = |ch: char| ch == '_' || ch.is_ascii_alphanumeric();
        let start = line[..byte.min(line.len())]
            .rfind(|ch: char| !is_symbol(ch))
            .map_or(0, |i| i + 1);
        let end = line[byte.min(line.len())..]
            .find(|ch: char| !is_symbol(ch))
            .map_or(line.len(), |i| byte.min(line.len()) + i);
        let name = &line[start..end];
        let Ok(assembly) = sceneasm_core::assemble(&text, sceneasm_core::Target::c64()) else {
            return Ok(None);
        };
        let Some(definition) = assembly.symbol_definitions.get(name) else {
            return Ok(None);
        };

        let mut locations = Vec::new();
        for (line_index, source_line) in text.lines().enumerate() {
            let code = source_line.split(';').next().unwrap_or("");
            let bytes = code.as_bytes();
            let mut index = 0;
            while index < bytes.len() {
                let ch = bytes[index] as char;
                if ch == '_' || ch.is_ascii_alphabetic() {
                    let token_start = index;
                    index += 1;
                    while index < bytes.len() {
                        let ch = bytes[index] as char;
                        if ch == '_' || ch.is_ascii_alphanumeric() {
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    if &code[token_start..index] == name {
                        let is_declaration = line_index + 1 == definition.line
                            && token_start + 1 == definition.column_start;
                        if params.context.include_declaration || !is_declaration {
                            locations.push(Location {
                                uri: uri.clone(),
                                range: Range {
                                    start: Position::new(line_index as u32, token_start as u32),
                                    end: Position::new(line_index as u32, index as u32),
                                },
                            });
                        }
                    }
                } else {
                    index += 1;
                }
            }
        }
        Ok(Some(locations))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents
            .write()
            .await
            .insert(uri.clone(), text.clone());
        self.analyze(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri;
            self.documents
                .write()
                .await
                .insert(uri.clone(), change.text.clone());
            self.analyze(uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().await.remove(&uri);
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: RwLock::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
