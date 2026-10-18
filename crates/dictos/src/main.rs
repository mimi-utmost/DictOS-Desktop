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
        Command::Lookup { word, dict } => lookup(&word, &dict),
        Command::Suggest {
            prefix,
            dict,
            limit,
        } => suggest(&prefix, &dict, limit),
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
