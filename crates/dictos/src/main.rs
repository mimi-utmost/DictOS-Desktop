use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use dictos_core::{DictError, LookupResult};
use dictos_dictionary::{import_jsonl, Dictionary, DictionaryBuilder};

#[derive(Parser)]
#[command(name = "dictos", version, about = "Dicionário popup offline-first")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Constrói um dicionário no formato v1 a partir de um JSONL de intercâmbio.
    Import {
        /// Arquivo JSONL de entrada (uma entrada por linha).
        input: PathBuf,
        /// Diretório de saída do dicionário.
        #[arg(long)]
        out: PathBuf,
    },
    /// Importa um diretório `dict/` do Princeton WordNet 3.1 para o formato v1.
    ImportWordnet {
        /// Diretório `dict/` do WordNet (com index.* e data.*).
        dict_dir: PathBuf,
        /// Diretório de saída do dicionário.
        #[arg(long)]
        out: PathBuf,
    },
    /// Consulta uma palavra no dicionário.
    Lookup {
        /// Palavra a consultar.
        word: String,
        /// Diretório do dicionário.
        #[arg(long)]
        dict: PathBuf,
    },
    /// Sugere headwords por prefixo (autocomplete).
    Suggest {
        /// Prefixo de busca.
        prefix: String,
        /// Diretório do dicionário.
        #[arg(long)]
        dict: PathBuf,
        /// Máximo de sugestões.
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Sobe o daemon: hotkey global → captura de seleção → popup de consulta.
    Run {
        /// Diretório do dicionário.
        #[arg(long)]
        dict: PathBuf,
        /// Atalho global (ex.: "Ctrl+Shift+D").
        #[arg(long, default_value = "Ctrl+Shift+D")]
        hotkey: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Import { input, out } => import(&input, &out),
        Command::ImportWordnet { dict_dir, out } => import_wordnet(&dict_dir, &out),
        Command::Lookup { word, dict } => lookup(&word, &dict),
        Command::Suggest {
            prefix,
            dict,
            limit,
        } => suggest(&prefix, &dict, limit),
        Command::Run { dict, hotkey } => run_daemon(&dict, &hotkey),
    }
}

fn import(input: &PathBuf, out: &PathBuf) -> Result<()> {
    let mut entries = Vec::new();
    for result in import_jsonl(input) {
        match result {
            Ok(entry) => entries.push(entry),
            Err(err) => bail!("falha na importação de {}: {err}", input.display()),
        }
    }
    DictionaryBuilder::build(entries, out)
        .with_context(|| format!("falha ao construir dicionário em {}", out.display()))?;
    println!("Dicionário construído em {}", out.display());
    Ok(())
}

fn import_wordnet(dict_dir: &PathBuf, out: &PathBuf) -> Result<()> {
    let entries = dictos_dictionary::wordnet::import_wordnet(dict_dir)
        .with_context(|| format!("falha ao importar WordNet de {}", dict_dir.display()))?;
    let total = entries.len();
    DictionaryBuilder::build(entries, out)
        .with_context(|| format!("falha ao construir dicionário em {}", out.display()))?;
    println!("{total} entradas importadas; dicionário construído em {}", out.display());
    Ok(())
}

fn lookup(word: &str, dict: &PathBuf) -> Result<()> {
    let dict = Dictionary::open(dict)
        .with_context(|| format!("falha ao abrir dicionário em {}", dict.display()))?;
    match dict.lookup(word) {
        Ok(result) => {
            print!("{}", format_result(&result));
            Ok(())
        }
        Err(DictError::NotFound(word)) => bail!("palavra desconhecida: {word}"),
        Err(err) => Err(err.into()),
    }
}

fn suggest(prefix: &str, dict: &PathBuf, limit: usize) -> Result<()> {
    let dict = Dictionary::open(dict)
        .with_context(|| format!("falha ao abrir dicionário em {}", dict.display()))?;
    for suggestion in dict.lookup_prefix(prefix, limit) {
        println!("{suggestion}");
    }
    Ok(())
}

fn format_result(result: &LookupResult) -> String {
    let entry = &result.entry;
    let mut out = String::new();
    out.push_str(&entry.word);
    out.push('\n');
    if result.diacritic_fallback {
        out.push_str("(encontrado por busca sem acentos)\n");
    }
    let mut n = 1;
    for sense in &entry.senses {
        out.push_str(&format!("{}. ({})\n   {}\n", n, sense.pos, sense.definition));
        n += 1;
        for example in &sense.examples {
            out.push_str(&format!("   • {example}\n"));
        }
    }
    if !entry.translations.is_empty() {
        out.push_str("Traduções:\n");
        for t in &entry.translations {
            out.push_str(&format!("  {}: {}\n", t.lang, t.word));
        }
    }
    out
}


#[cfg(windows)]
fn run_daemon(dict_dir: &PathBuf, hotkey: &str) -> Result<()> {
    use dictos_platform_windows as win;
    use dictos_popup::{slint, SlintPopup};

    let dictionary = Dictionary::open(dict_dir)
        .with_context(|| format!("falha ao abrir dicionário em {}", dict_dir.display()))?;
    let parsed = win::hotkey::parse_hotkey(hotkey).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let (_manager, receiver) =
        win::hotkey::register(&parsed).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    eprintln!("DictOS ativo — hotkey: {hotkey}. Selecione texto e pressione o atalho. Ctrl+C para sair.");

    let popup = SlintPopup::new().context("falha ao criar popup")?;
    let weak = popup.as_weak();

    std::thread::spawn(move || {
        let uia = win::uia::UiaSelectionProvider;
        for event in receiver.iter() {
            if event.state != win::hotkey::HotKeyState::Pressed {
                continue;
            }
            eprintln!("[dictos] hotkey acionado");
            let outcome = capture_and_lookup(&uia, &dictionary);
            match &outcome {
                Ok(view) => eprintln!("[dictos] lookup ok: {}", view.word),
                Err((title, _)) => eprintln!("[dictos] {title}"),
            }
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                eprintln!("[dictos] exibindo popup");
                if let Some(ui) = weak.upgrade() {
                    match outcome {
                        Ok(view) => dictos_popup::show_lookup_ui(&ui, &view),
                        Err((title, message)) => {
                            dictos_popup::show_message_ui(&ui, &title, &message)
                        }
                    }
                    eprintln!("[dictos] popup exibido");
                } else {
                    eprintln!("[dictos] ERRO: weak do popup não upgradou");
                }
            });
        }
    });

    eprintln!("[dictos] event loop iniciando");
    // Variante para daemons: o loop continua mesmo com o popup escondido.
    let loop_result = slint::run_event_loop_until_quit();
    eprintln!("[dictos] event loop encerrou: {loop_result:?}");
    loop_result.context("event loop do Slint falhou")?;
    Ok(())
}

#[cfg(windows)]
fn capture_and_lookup(
    uia: &dyn dictos_selection::SelectionProvider,
    dictionary: &Dictionary,
) -> std::result::Result<dictos_popup::LookupView, (String, String)> {
    use dictos_platform_windows as win;
    use dictos_selection::{extract_query, SelectionError};

    let captured = match uia.capture() {
        Ok(selection) => Ok(selection),
        Err(SelectionError::NoSelection) => win::clipboard_fallback::capture_via_clipboard(),
        Err(other) => Err(other),
    };

    let selection = captured.map_err(|err| match err {
        SelectionError::NoSelection => (
            "Nada para consultar".to_string(),
            "Nenhum texto selecionado, ou o aplicativo não expõe a seleção.".to_string(),
        ),
        SelectionError::ClipboardRestoreFailed => (
            "Consulta cancelada".to_string(),
            "Não foi possível restaurar sua área de transferência; a consulta foi abortada para não perder seus dados.".to_string(),
        ),
        SelectionError::CaptureFailed(msg) => ("Falha na captura".to_string(), msg),
    })?;

    let query = extract_query(&selection.text).ok_or_else(|| {
        (
            "Seleção longa demais".to_string(),
            "Selecione uma palavra ou expressão curta (até 5 palavras).".to_string(),
        )
    })?;

    match dictionary.lookup(&query) {
        Ok(result) => Ok(dictos_popup::LookupView::from(&result)),
        Err(DictError::NotFound(_)) => Err((
            "Palavra desconhecida".to_string(),
            format!("'{query}' não está no dicionário offline."),
        )),
        Err(err) => Err(("Erro no dicionário".to_string(), err.to_string())),
    }
}

#[cfg(not(windows))]
fn run_daemon(_dict_dir: &PathBuf, _hotkey: &str) -> Result<()> {
    bail!("o daemon está disponível apenas no Windows nesta etapa (Linux chega na Etapa 7)");
}
