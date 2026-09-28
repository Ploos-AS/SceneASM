use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
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

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.analyze(params.text_document.uri, &params.text_document.text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.analyze(params.text_document.uri, &change.text).await;
        }
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
