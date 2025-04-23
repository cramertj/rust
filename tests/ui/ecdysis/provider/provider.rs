//@ check-pass

#[provider]
type Provided;

fn main() {
    let provided: Provided = ();
    provided
}
