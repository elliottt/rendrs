
extern crate rendrs;
extern crate eliza_error;

use eliza_error::Error;

use rendrs::film;

fn main() -> Result<(), Error> {

    let f = film::Film::new(
        film::Resolution::new(100, 100),
        [0.2, 0.2, 0.8, 0.8].into(),
        rendrs::filter::box_(),
    );

    println!("{:?}", f.cropped_bounds);

    Ok(())
}
