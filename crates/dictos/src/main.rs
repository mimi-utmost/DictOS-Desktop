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
        /// Atalho global (ex.: "Ctrl+Shift+D"). Tem precedência sobre o
        /// configurado em `config set hotkey`.
        #[arg(long)]
        hotkey: Option<String>,
    },
    /// Lê e altera preferências persistidas.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Histórico opcional de consultas (desligado por padrão).
    History {
        #[command(subcommand)]
        action: HistoryAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Lê o valor efetivo de uma chave (ex.: hotkey).
    Get { key: String },
    /// Define uma chave (ex.: config set hotkey "Ctrl+Alt+D").
    Set { key: String, value: String },
    /// Lista todas as chaves efetivas (defaults + definidas).
    List,
}

#[derive(Subcommand)]
enum HistoryAction {
    /// Lista as consultas mais recentes.
    List {
        /// Máximo de entradas.
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Apaga todo o histórico.
    Clear,
    /// Liga o registro de histórico (opt-in).
    On,
    /// Desliga o registro de histórico.
    Off,
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
        Command::Run { dict, hotkey } => run_daemon(&dict, hotkey.as_deref()),
        Command::Config { action } => config(action),
        Command::History { action } => history_cmd(action),
    }
}

fn open_settings() -> Result<dictos_settings::Settings> {
    dictos_settings::Settings::open()
        .with_context(|| "falha ao abrir o banco de preferências".to_string())
}

fn config(action: ConfigAction) -> Result<()> {
    use dictos_settings::DEFAULTS;

    let settings = open_settings()?;
    match action {
        ConfigAction::Get { key } => println!("{}", settings.get_effective(&key)),
        ConfigAction::Set { key, value } => {
            settings.set(&key, &value)?;
            println!("{key} = {value}");
        }
        ConfigAction::List => {
            let explicit: std::collections::HashMap<String, String> =
                settings.list()?.into_iter().collect();
            let mut keys: Vec<&str> = DEFAULTS.iter().map(|(k, _)| *k).collect();
            for key in explicit.keys() {
                if !keys.contains(&key.as_str()) {
                    keys.push(key);
                }
            }
            keys.sort();
            for key in keys {
                let value = explicit
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| settings.get_effective(key));
                println!("{key} = {value}");
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn history_cmd(action: HistoryAction) -> Result<()> {
    use dictos_history::History;
    use dictos_platform_windows::dpapi::DpapiProtector;

    match action {
        HistoryAction::On | HistoryAction::Off => {
            let settings = open_settings()?;
            let enabled = matches!(action, HistoryAction::On);
            settings.set(
                dictos_settings::KEY_HISTORY_ENABLED,
                if enabled { "true" } else { "false" },
            )?;
            println!(
                "histórico {} ({})."
                ,
                if enabled { "ligado" } else { "desligado" },
                if enabled {
                    "as palavras consultadas serão registradas, criptografadas, neste computador"
                } else {
                    "nada novo será registrado; use `dictos history clear` para apagar o existente"
                }
            );
            Ok(())
        }
        HistoryAction::List { limit } => {
            let history = History::open(dictos_settings::db_path(), &DpapiProtector)?;
            for entry in history.list(limit)? {
                println!(
                    "{}  {:<24}  {}",
                    dictos_history::format_utc(entry.looked_up_at),
                    entry.word,
                    if entry.found { "encontrada" } else { "desconhecida" },
                );
            }
            Ok(())
        }
        HistoryAction::Clear => {
            let history = History::open(dictos_settings::db_path(), &DpapiProtector)?;
            history.clear()?;
            println!("histórico apagado.");
            Ok(())
        }
    }
}

#[cfg(not(windows))]
fn history_cmd(_action: HistoryAction) -> Result<()> {
    bail!("o histórico depende de criptografia do sistema (DPAPI); só Windows nesta etapa");
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
fn run_daemon(dict_dir: &PathBuf, hotkey_flag: Option<&str>) -> Result<()> {
    use dictos_history::History;
    use dictos_platform_windows as win;
    use dictos_popup::{slint, PlacementContext, SlintPopup};
    use dictos_settings::{Settings, KEY_FONT_SCALE, KEY_HISTORY_ENABLED, KEY_HOTKEY};

    // DPI Per-Monitor V2: todas as coordenadas (âncora UIA, cursor, work
    // area, posição do popup) passam a ser pixels físicos reais.
    win::monitor::set_per_monitor_v2_awareness();

    let settings = Settings::open().context("falha ao abrir o banco de preferências")?;
    let hotkey = match hotkey_flag {
        Some(flag) => flag.to_string(),
        None => settings.get_effective(KEY_HOTKEY),
    };
    let history_enabled = settings.get_or(KEY_HISTORY_ENABLED, "false") == "true";
    let font_scale: f32 = settings
        .get_effective(KEY_FONT_SCALE)
        .parse()
        .unwrap_or(1.0);

    let dictionary = Dictionary::open(dict_dir)
        .with_context(|| format!("falha ao abrir dicionário em {}", dict_dir.display()))?;
    let parsed = win::hotkey::parse_hotkey(&hotkey).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let (_manager, receiver) =
        win::hotkey::register(&parsed).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    eprintln!(
        "DictOS ativo — hotkey: {hotkey}. Histórico: {}. Selecione texto e pressione o atalho.",
        if history_enabled { "ligado" } else { "desligado" }
    );

    let popup = SlintPopup::new().context("falha ao criar popup")?;
    popup.set_font_scale(font_scale);
    let weak = popup.as_weak();

    std::thread::spawn(move || {
        let uia = win::uia::UiaSelectionProvider;
        let protector = win::dpapi::DpapiProtector;
        let history = history_enabled.then(|| {
            History::open(dictos_settings::db_path(), &protector)
                .map_err(|e| eprintln!("[dictos] histórico indisponível: {e}"))
                .ok()
        });
        let history = history.flatten();

        for event in receiver.iter() {
            if event.state != win::hotkey::HotKeyState::Pressed {
                continue;
            }
            eprintln!("[dictos] hotkey acionado");
            let outcome = capture_and_lookup(&uia, &dictionary);
            match &outcome.result {
                Ok(view) => eprintln!("[dictos] lookup ok: {}", view.word),
                Err((title, _)) => eprintln!("[dictos] {title}"),
            }
            if let (Some(history), Some(query)) = (&history, &outcome.query) {
                if let Err(e) = history.record(query, outcome.result.is_ok()) {
                    eprintln!("[dictos] falha ao registrar histórico: {e}");
                }
            }
            let placement = outcome.anchor.map(|anchor| PlacementContext {
                anchor,
                work_area: win::monitor::work_area_containing(Some(anchor)),
            });
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                eprintln!("[dictos] exibindo popup");
                if let Some(ui) = weak.upgrade() {
                    match outcome.result {
                        Ok(view) => dictos_popup::show_lookup_ui(&ui, &view, placement.as_ref()),
                        Err((title, message)) => {
                            dictos_popup::show_message_ui(&ui, &title, &message, placement.as_ref())
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

/// Resultado de um ciclo hotkey → captura → lookup, com a âncora da seleção
/// (mesmo quando a consulta falha, para posicionar a mensagem de erro) e a
/// consulta normalizada (para o histórico, quando habilitado).
#[cfg(windows)]
struct LookupOutcome {
    result: std::result::Result<dictos_popup::LookupView, (String, String)>,
    anchor: Option<dictos_selection::ScreenRect>,
    query: Option<String>,
}

#[cfg(windows)]
fn capture_and_lookup(
    uia: &dyn dictos_selection::SelectionProvider,
    dictionary: &Dictionary,
) -> LookupOutcome {
    use dictos_platform_windows as win;
    use dictos_selection::{extract_query, SelectionError};

    let policy = win::app_policy::foreground_policy();
    if policy.is_own_process {
        return LookupOutcome {
            result: Err((
                "Nada para consultar".to_string(),
                "A janela em foco é o próprio DictOS.".to_string(),
            )),
            anchor: None,
            query: None,
        };
    }

    let captured = match (policy.allow_uia, uia.capture()) {
        (_, Ok(selection)) => Ok(selection),
        (true, Err(SelectionError::NoSelection)) if policy.allow_clipboard_fallback => {
            win::clipboard_fallback::capture_via_clipboard()
        }
        (true, Err(SelectionError::NoSelection)) => {
            return LookupOutcome {
                result: Err((
                    "Captura indisponível".to_string(),
                    "Este aplicativo não expõe a seleção e não permite captura segura por Ctrl+C.".to_string(),
                )),
                anchor: None,
                query: None,
            };
        }
        (false, _) => {
            return LookupOutcome {
                result: Err((
                    "Captura indisponível".to_string(),
                    "Este aplicativo não permite captura de seleção.".to_string(),
                )),
                anchor: None,
                query: None,
            };
        }
        (_, Err(other)) => Err(other),
    };

    let selection = match captured {
        Ok(selection) => selection,
        Err(err) => {
            let message = match err {
                SelectionError::NoSelection => (
                    "Nada para consultar".to_string(),
                    "Nenhum texto selecionado, ou o aplicativo não expõe a seleção.".to_string(),
                ),
                SelectionError::ClipboardRestoreFailed => (
                    "Consulta cancelada".to_string(),
                    "Não foi possível restaurar sua área de transferência; a consulta foi abortada para não perder seus dados.".to_string(),
                ),
                SelectionError::CaptureFailed(msg) => ("Falha na captura".to_string(), msg),
            };
            return LookupOutcome {
                result: Err(message),
                anchor: None,
                query: None,
            };
        }
    };

    let anchor = selection.anchor;
    let query = extract_query(&selection.text);
    let result = (|| {
        let query = query.clone().ok_or_else(|| {
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
    })();

    LookupOutcome { result, anchor, query }
}

#[cfg(not(windows))]
fn run_daemon(_dict_dir: &PathBuf, _hotkey: Option<&str>) -> Result<()> {
    bail!("o daemon está disponível apenas no Windows nesta etapa (Linux chega na Etapa 7)");
}
