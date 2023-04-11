extern crate alloc;

use derive_builder::Builder;

// #[derive(Builder)]
// pub struct Command {
//     executable: String,
//     #[builder(each = "arg")]
//     args: Vec<String>,
//     env: Vec<String>,
//     current_dir: Option<String>,
// }

#[derive(Builder)]
pub struct Command {
    #[builder(eac = "arg")]
    args: Vec<String>,
}

fn main() {}