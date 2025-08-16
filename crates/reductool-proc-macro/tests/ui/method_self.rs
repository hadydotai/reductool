use reductool_proc_macro::aitool;

struct S;

impl S {
    #[aitool]
    fn m(&self, x: i32) {}
}

fn main() {}
