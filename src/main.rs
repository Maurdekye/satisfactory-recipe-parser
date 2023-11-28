use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::{self, File},
};

#[allow(non_snake_case)]
#[derive(Deserialize, Clone)]
struct DocsClass {
    NativeClass: String,
    Classes: Vec<HashMap<String, Value>>,
}

#[derive(Serialize, Clone)]
struct Recipe {
    machine: String,
    ingredients: Vec<(String, f64)>,
    products: Vec<(String, f64)>,
}

#[derive(Deserialize, Clone)]
struct Config {
    item_native_class_names: Vec<String>,
    machine_native_class_names: Vec<String>,
}

struct LexicalParser<'a> {
    target: &'a str,
    index: usize,
    stack: Vec<usize>,
}

impl<'a> From<&'a str> for LexicalParser<'a> {
    fn from(value: &'a str) -> Self {
        LexicalParser {
            target: value,
            index: 0,
            stack: Vec::new(),
        }
    }
}

impl<'a> Iterator for LexicalParser<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let delimiter_re: Regex = Regex::new(r"[()]").unwrap();

        loop {
            match delimiter_re.find_at(self.target, self.index) {
                None => return None,
                Some(re_match) => {
                    self.index = re_match.end();
                    match re_match.as_str() {
                        "(" => self.stack.push(re_match.start()),
                        ")" => {
                            return Some((
                                self.stack.len(),
                                &self.target[self.stack.pop().unwrap()..self.index],
                            ))
                        }
                        _ => panic!("??? regex machine broke??"),
                    }
                }
            }
        }
    }
}

fn parse_item_list(raw_list: &str) -> Vec<(String, usize)> {
    let itemclass_re = Regex::new("'\".*/[^/]*\\.([^/]*)\"'").unwrap();
    LexicalParser::from(raw_list)
        .filter_map(|(depth, capture)| {
            if depth == 2 {
                let obj = capture[1..capture.len() - 1]
                    .split(",")
                    .map(|field| field.split_once("=").unwrap())
                    .collect::<HashMap<_, _>>();
                let full_name = obj["ItemClass"].to_string();
                let short_name = itemclass_re
                    .captures(full_name.as_str())
                    .unwrap()
                    .get(1)
                    .unwrap()
                    .as_str()
                    .to_string();
                Some((short_name, obj["Amount"].parse().unwrap()))
            } else {
                None
            }
        })
        .collect()
}

fn round_to(x: f64, d: u32) -> f64 {
    let factor = 10usize.pow(d) as f64;
    (x * factor).round() / factor
}

/// Satisfactory Recipe Importer
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Docs.json file to parse
    #[arg(default_value = "Docs.json")]
    docs_file: String,

    /// Config file
    #[arg(long, short, default_value = "config.json")]
    config_file: String,

    /// Output filename
    #[arg(long, short, default_value = "recipes.json")]
    output: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let config =
        serde_json::from_str::<Config>(fs::read_to_string(args.config_file.to_string())?.as_str())?;

    println!("Loading {}", args.docs_file);
    let contents = utf16_reader::read_to_string(File::open(args.docs_file)?);

    let json: Vec<DocsClass> = serde_json::from_str(contents.as_str())?;

    let native_class_re = Regex::new(r"'.*\.([^/]*)'").unwrap();
    let class_map: HashMap<String, Vec<HashMap<String, Value>>> = json
        .into_iter()
        .map(
            |DocsClass {
                 NativeClass,
                 Classes,
             }| {
                (
                    native_class_re
                        .captures(NativeClass.as_str())
                        .expect("Name does not match capture format!")
                        .get(1)
                        .unwrap()
                        .as_str()
                        .to_string(),
                    Classes,
                )
            },
        )
        .collect();

    let collect_classes = |classes: Vec<String>| {
        classes
            .into_iter()
            .flat_map(|native_class_name| class_map[&native_class_name].clone())
            .collect::<Vec<_>>()
    };

    println!("Parsing contents");
    let item_info_by_id: HashMap<String, (String, bool)> =
        collect_classes(config.item_native_class_names)
            .into_iter()
            .map(|class| {
                (
                    class["ClassName"].as_str().unwrap().to_string(),
                    (
                        class["mDisplayName"].as_str().unwrap().to_string(),
                        class.get("mForm").map_or(false, |form| form == "RF_LIQUID"),
                    ),
                )
            })
            .collect();

    let machine_name_id_pairs: Vec<(String, String)> =
        collect_classes(config.machine_native_class_names)
            .into_iter()
            .map(|class| {
                (
                    class["ClassName"].as_str().unwrap().to_string(),
                    class["mDisplayName"].as_str().unwrap().to_string(),
                )
            })
            .collect();

    println!("Constructing Recipes");
    let (alternates, defaults): (Vec<_>, Vec<_>) = class_map["FGRecipe"]
        .clone()
        .into_iter()
        .filter_map(|class| {
            machine_name_id_pairs
                .iter()
                .find(|(id, _)| class["mProducedIn"].as_str().unwrap().contains(id))
                .map(|(_, machine)| {
                    let duration = class["mManufactoringDuration"]
                        .as_str()
                        .unwrap()
                        .parse::<f64>()
                        .unwrap()
                        / 60.0;
                    let reformat_item_list = |item_list: &str| {
                        parse_item_list(item_list)
                            .into_iter()
                            .map(|(id, quantity)| {
                                let (name, is_liquid) = item_info_by_id
                                    .get(&id)
                                    .unwrap_or_else(|| panic!("name for id '{}' not found!", id));
                                (
                                    name.to_string(),
                                    round_to(
                                        ((quantity as f64) / duration)
                                            * if *is_liquid { 1e-3 } else { 1.0 },
                                        3,
                                    ),
                                )
                            })
                            .collect()
                    };
                    (
                        Recipe {
                            machine: machine.clone(),
                            ingredients: reformat_item_list(
                                &class["mIngredients"].as_str().unwrap(),
                            ),
                            products: reformat_item_list(&class["mProduct"].as_str().unwrap()),
                        },
                        class
                            .get("mDisplayName")
                            .map_or(false, |recipe_name_value| {
                                recipe_name_value.as_str().map_or(false, |recipe_name| {
                                    recipe_name.starts_with("Alternate")
                                })
                            }),
                    )
                })
        })
        .partition(|(_, alternate)| *alternate);

    let recipes: Vec<Recipe> = defaults
        .into_iter()
        .chain(alternates)
        .map(|(recipe, _)| recipe)
        .collect();

    println!("Saving to {}", args.output);
    let output_json = serde_json::to_string_pretty(&recipes)?;

    fs::write(args.output, output_json)?;

    Ok(())
}
