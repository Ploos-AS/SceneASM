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
                        sceneasm_core::diagnostic::Severity::Info => DiagnosticSeverity::INFORMATION,
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
        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "SceneASM".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client.log_message(MessageType::INFO, "SceneASM LSP ready").await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let text = {
            let documents = self.documents.read().await;
            let Some(text) = documents.get(&uri) else { return Ok(None); };
            text.clone()
        };
        let Ok(assembly) = sceneasm_core::assemble(&text, sceneasm_core::Target::c64()) else { return Ok(None); };
        let line = position.line as usize + 1;
        let Some(instruction) = assembly.instructions.iter().find(|instruction| {
            instruction.source.line == line
                && position.character as usize + 1 >= instruction.source.column_start
                && position.character as usize + 1 <= instruction.source.column_end
        }) else { return Ok(None); };
        let extra = match instruction.opcode.extra_cycle {
            sceneasm_core::ExtraCycle::None => 0,
            sceneasm_core::ExtraCycle::PageCross | sceneasm_core::ExtraCycle::BranchTaken => 1,
            sceneasm_core::ExtraCycle::BranchTakenAndPageCross => 2,
        };
        let base = instruction.opcode.cycles as u16;
        let worst = base + extra;
        let value = format!(
            "**{}**  \\nAddress: `${:04x}`  \\nMode: `{:?}`  \\nCycles: **{}**{}",
            instruction.opcode.mnemonic,
            instruction.address,
            instruction.opcode.mode,
            base,
            if worst == base { String::new() } else { format!("–{}", worst) }
        );
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value }),
            range: None,
        }))
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let text = {
            let documents = self.documents.read().await;
            let Some(text) = documents.get(&uri) else { return Ok(None); };
            text.clone()
        };
        let line = text.lines().nth(position.line as usize).unwrap_or("");
        let byte = position.character as usize;
        let is_symbol = |ch: char| ch == '_' || ch.is_ascii_alphanumeric();
        let start = line[..byte.min(line.len())].rfind(|ch: char| !is_symbol(ch)).map_or(0, |i| i + 1);
        let end = line[byte.min(line.len())..].find(|ch: char| !is_symbol(ch)).map_or(line.len(), |i| byte.min(line.len()) + i);
        let name = &line[start..end];
        let Ok(assembly) = sceneasm_core::assemble(&text, sceneasm_core::Target::c64()) else { return Ok(None); };
        let Some(span) = assembly.symbol_definitions.get(name) else { return Ok(None); };
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: Range {
                start: Position::new((span.line - 1) as u32, (span.column_start - 1) as u32),
                end: Position::new((span.line - 1) as u32, (span.column_end - 1) as u32),
            },
        })))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.write().await.insert(uri.clone(), text.clone());
        self.analyze(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri;
            self.documents.write().await.insert(uri.clone(), change.text.clone());
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
