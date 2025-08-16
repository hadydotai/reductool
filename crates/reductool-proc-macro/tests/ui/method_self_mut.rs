use reductool_proc_macro::aitool;

struct S;

impl S {
    #[aitool]
    fn m(&mut self, x: i32) {}
}

fn main() {}
