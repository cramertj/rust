//@ run-pass

#[provider(id = "return_arg_ty")]
type Provided;

fn main() {
    let provided: Provided<i32> = 32;
    let _provided: i32 = provided;
    let provided: Provided<usize> = "12345".parse().unwrap();
    let provided: usize = provided;
    assert_eq!(provided, 12345);

    let nested: Provided<Provided<u8>> = 6;
    let _nested: u8 = nested;
}
