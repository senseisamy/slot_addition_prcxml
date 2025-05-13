use std::{
    collections::HashMap,
    fs,
    io::{Cursor, Result},
    path::Path,
};

// embed the ui_chara_db.prc file in the executable, the file must be in the src directory
const UI_CHARA_DB_PRC: &[u8] = include_bytes!("ui_chara_db.prc");

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    let verbose = if args.contains(&"-v".to_string()) {
        args.retain(|s| s != "-v");
        true
    } else {
        false
    };
    
    if args.len() != 2 {
        println!("usage: ./additional_slot_db /path/to/mods/directory");
        return;
    }

    let mods_directory = Path::new(&args[1]);
    let mut fighter_max_slot: HashMap<String, u8> = HashMap::new();

    match parse_max_slot(mods_directory, &mut fighter_max_slot) {
        Err(e) => eprintln!("Error: {e}"),
        Ok(_) => {
            if verbose {
                print_fighters_max_slot(&fighter_max_slot);
            }
            generate_prcxml(mods_directory, &mut fighter_max_slot);
        }
    }
}

fn parse_max_slot(mods_directory: &Path, fighter_max_slot: &mut HashMap<String, u8>) -> Result<()> {
    iterate_mods(mods_directory, fighter_max_slot)?;
    handle_special_names(fighter_max_slot);
    Ok(())
}

fn generate_prcxml(mods_directory: &Path, fighter_max_slot: &mut HashMap<String, u8>) {
    let mut reader = Cursor::new(UI_CHARA_DB_PRC);
    match prcx::read_stream(&mut reader) {
        Err(_) => eprintln!("Error: missing ui_chara_db.prc file"),
        Ok(source) => {
            let mut modded = source.clone();
            if let prcx::ParamKind::List(db_root) = &mut modded.0[0].1 {
                for elem in &mut db_root.0 {
                    change_value_in_prc(elem, &fighter_max_slot);
                }
            }
            gen_xml_diff(&source, &modded, mods_directory);
        }
    }
}

fn print_fighters_max_slot(fighter_max_slot: &HashMap<String, u8>) {
    println!("Here is each fighter affected with their new maximum slot:");
    for (fighter, max) in fighter_max_slot {
        println!(" {fighter} - c{:0>2}", max - 1)
    }
    println!()
}

fn iterate_mods(mods_directory: &Path, fighter_max_slot: &mut HashMap<String, u8>) -> Result<()> {
    for entry in fs::read_dir(mods_directory)? {
        let path = entry?.path();
        if path.is_dir() {
            let path = path.join("fighter");
            if path.exists() {
                figher_check_max(&path, fighter_max_slot)?
            }
        }
    }

    Ok(())
}

fn figher_check_max(
    mods_directory: &Path,
    fighter_max_slot: &mut HashMap<String, u8>,
) -> Result<()> {
    for entry in fs::read_dir(mods_directory)? {
        let path = entry?.path();
        if path.is_dir() {
            let fighter = path.file_name().unwrap().to_str().unwrap().to_string();
            let body = path.join("model/body");
            let diver = path.join("model/diver");
            if body.exists() || diver.exists() {
                let path = if body.exists() { body } else { diver };
                for entry in fs::read_dir(path)? {
                    let path = entry?.path();
                    if path.is_dir() && path.file_name().unwrap().to_str().unwrap().starts_with("c")
                    {
                        let slot_number = path.file_name().unwrap().to_str().unwrap()[1..]
                            .parse::<u8>()
                            .unwrap()
                            + 1;

                        fighter_max_slot
                            .entry(fighter.clone())
                            .and_modify(|max| {
                                if *max < slot_number {
                                    *max = slot_number
                                }
                            })
                            .or_insert(slot_number);
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_special_names(fighter_max: &mut HashMap<String, u8>) {
    if fighter_max.contains_key("element") {
        let max = *fighter_max.get("element").unwrap();
        fighter_max.insert("eflame_first".to_string(), max);
        fighter_max.insert("elight_first".to_string(), max);
        fighter_max.insert("eflame_only".to_string(), max);
        fighter_max.insert("elight_only".to_string(), max);
    }
}

fn change_value_in_prc(fighter: &mut prcx::ParamKind, fighter_max: &HashMap<String, u8>) {
    if let prcx::ParamKind::Struct(fighter) = fighter {
        if let prcx::ParamKind::Str(name) = &mut fighter.0[1].1 {
            if let Some(value) = fighter_max.get(name) {
                fighter.0[33].1 = prcx::ParamKind::U8(*value);
            }
        }
    }
}

fn gen_xml_diff(source: &prcx::ParamStruct, modded: &prcx::ParamStruct, path: &Path) {
    let diff = prcx::generate_patch(&source, &modded).unwrap();
    match diff {
        Some(diff) => {
            let dir = path.join("(UI) Additional Slots/ui/param/database");
            if !dir.exists() {
                fs::create_dir_all(&dir).unwrap();
            }
            let mut file = std::io::BufWriter::new(
                std::fs::File::create(dir.join("ui_chara_db.prcxml")).unwrap(),
            );
            match prcx::write_xml(&diff, &mut file) {
                Ok(_) => println!("Successfuly generated xml diff !"),
                Err(_) => eprint!("Error: failed to create xml diff"),
            }
        }
        None => println!("Error: no fighter with additional slots were found"),
    }
}
