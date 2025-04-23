#[provider(id = "some_magic_id")]
type Provided;

fn main() {
    let provided: Provided<i32> = ();
    //~^ ERROR provided types are not yet supported
    provided
}
