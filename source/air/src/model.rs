use crate::ast::{Decl, DeclX, Ident, SnapShots, Typ, TypX};
use crate::context::Context;
use std::collections::HashMap;
use std::fmt;
use z3::ast::{Bool, Dynamic, Int};
//use z3::{FuncDecl, Sort};

#[derive(Debug)]
pub struct Model<'a> {
    z3_model: z3::Model<'a>,
    id_snapshots: SnapShots,
    value_snapshots: HashMap<Ident, HashMap<Ident, String>>,
}

// TODO: Duplicated from smt_verify
fn new_const<'ctx>(context: &mut Context<'ctx>, name: &String, typ: &Typ) -> Dynamic<'ctx> {
    match &**typ {
        TypX::Bool => Bool::new_const(context.context, name.clone()).into(),
        TypX::Int => Int::new_const(context.context, name.clone()).into(),
        TypX::Named(x) => {
            let sort = &context.typs[x];
            let fdecl = z3::FuncDecl::new(context.context, name.clone(), &[], sort);
            fdecl.apply(&[])
        }
    }
}

impl<'a> Model<'a> {
    pub fn new(model: z3::Model<'a>, snapshots: SnapShots) -> Model<'a> {
        println!("Creating a new model with {} snapshots", snapshots.len());
        Model { z3_model: model, id_snapshots: snapshots, value_snapshots: HashMap::new() }
    }

    //    pub fn save_snapshots(&self, snapshots: SnapShots) {
    //        self.snapshots = snapshots.clone();
    //    }
    fn lookup_z3_var(&self, var_name: &String, var_smt: &Dynamic) -> String {
        if let Some(x) = self.z3_model.eval(var_smt) {
            if let Some(b) = x.as_bool() {
                format!("{}", b)
            } else if let Some(i) = x.as_int() {
                format!("{}", i)
            } else {
                println!("Unexpected type returned from model eval for {}", var_name);
                "".to_string()
            }
        } else {
            println!("Failed to extract evaluation of var {} from Z3's model", var_name);
            "".to_string()
        }
    }

    /// Reconstruct an AIR-level model based on the Z3 model
    pub fn build(&mut self, context: &mut Context, local_vars: Vec<Decl>) {
        println!("Building the AIR model");
        for (snap_id, id_snapshot) in &self.id_snapshots {
            let mut value_snapshot = HashMap::new();
            println!("Snapshot {} has {} variables", snap_id, id_snapshot.len());
            for (var_id, var_count) in &*id_snapshot {
                let var_name = crate::var_to_const::rename_var(&*var_id, *var_count);
                println!("\t{}", var_name);
                let var_smt = context
                    .vars
                    .get(&var_name)
                    .unwrap_or_else(|| panic!("internal error: variable {} not found", var_name));
                let val = self.lookup_z3_var(&var_name, var_smt);
                value_snapshot.insert(var_id.clone(), val);
            }
            // Add the local variables to every snapshot for uniformity
            println!("local_vars has {} variables", local_vars.len());
            for decl in local_vars.iter() {
                if let DeclX::Const(var_name, typ) = &**decl {
                    println!("\t{}", var_name);
                    let var_smt = new_const(context, &var_name, &typ);
                    let val = self.lookup_z3_var(&var_name, &var_smt);
                    value_snapshot.insert(var_name.clone(), val);
                    //value_snapshot.insert(Rc::new((*var_name).clone()), val);
                } else {
                    panic!("Expected local vars to all be constants at this point");
                }
            }
            self.value_snapshots.insert(snap_id.clone(), value_snapshot);
        }
    }

    pub fn query_variable(&self, snapshot: Ident, name: Ident) -> Option<String> {
        Some(self.value_snapshots.get(&snapshot)?.get(&name)?.to_string())
    }
}

impl<'a> fmt::Display for Model<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\nDisplaying model with {} snapshots\n", self.value_snapshots.len())?;
        for (snap_id, value_snapshot) in &self.value_snapshots {
            write!(f, "Snapshot <{}>:\n", snap_id)?;
            for (var_name, value) in &*value_snapshot {
                write!(f, "\t{} -> {}\n", var_name, value)?;
            }
        }
        Ok(())
    }
}
