mod error;
mod extension_manager;
mod io;
mod parser;
mod storage;

use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    error::DatabaseError, extension_manager::ExtensionManager, parser::Command, storage::Storage,
};

fn main() {
    let ext_manager = ExtensionManager::load("extensions");
    let storage = Rc::new(RefCell::new(Storage::new()));

    loop {
        let raw_input = match io::read() {
            Ok(Some(i)) => i,
            Ok(None) => break,
            Err(e) => {
                io::print_error(DatabaseError::from(e));
                continue;
            }
        };

        let mut cmd = match parser::parse(&raw_input) {
            Ok(c) => c,
            Err(e) => {
                io::print_error(&e);
                continue;
            }
        };

        let (k, v) = cmd.params_as_tuple();
        match ext_manager.trigger_pre_hook(&storage, cmd.as_str(), k, v) {
            Ok(new_value) => {
                if let (Command::Add { value, .. }, Some(nv)) = (&mut cmd, new_value) {
                    *value = nv;
                }
            }
            Err(e) => {
                io::print_error(&e);
                continue;
            }
        }

        match &cmd {
            Command::Add { key, value } => {
                storage.borrow_mut().add(key, value);

                let (k, v) = cmd.params_as_tuple();
                if let Err(e) = ext_manager.trigger_post_hook(&storage, cmd.as_str(), k, v, None) {
                    io::print_error(&e);
                    continue;
                }
                io::print_success();
            }

            Command::Get { key } => {
                let current_val = storage.borrow().get(key).map(str::to_string);
                match current_val {
                    Some(val) => {
                        let (k, v) = cmd.params_as_tuple();
                        match ext_manager.trigger_post_hook(
                            &storage,
                            cmd.as_str(),
                            k,
                            v,
                            Some(&val),
                        ) {
                            Ok(Some(formatted)) => io::print_value(&formatted),
                            Ok(None) => io::print_value(&val),
                            Err(e) => {
                                io::print_error(&e);
                                continue;
                            }
                        }
                    }
                    None => {
                        io::print_error(DatabaseError::NotFound);
                    }
                }
            }

            Command::Exit => break,
        }
    }
}
