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
    //executable: String,
    args: Vec<String>,
    //env: Vec<String>,
    //current_dir: String,
}

fn main() {}

mod test {
    use crate::Command;

    pub struct CommandBuilder {
        args: std::option::Option<Vec<String>>,
    }

    impl CommandBuilder {
        pub fn args(&mut self, v: impl Into<Vec<String>>) -> &mut Self {
            self.args = Some(v.into());
            self
        }
        pub fn build(&mut self) -> Result<Command, &'static str> {
            Ok(Command {
                args: self.args.take().unwrap_or(::alloc::vec::Vec::new()),
            })
        }
    }

    impl Command {
        pub fn builder() -> CommandBuilder {
            Default::default()
        }
    }
}