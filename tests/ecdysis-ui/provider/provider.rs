//@ run-pass

#[provider(id = "some_magic_id")]
type Provided;

fn main() {
    let provided: Provided<i32> = 32;
    let _provided: i32 = provided;
    let provided: Provided<usize> = "12345".parse().unwrap();
    let provided: usize = provided;
    assert_eq!(provided, 12345);
}
