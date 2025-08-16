use reductool_proc_macro::aitool;

struct S {
    x: i32,
    y: i32,
}

#[aitool]
fn f(S { x, y }: S) {}

fn main() {}
