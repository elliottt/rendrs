
use rendrs::canvas;

pub fn main() {

    let mut c = canvas::Canvas::new(512,512);

    for y in 0 .. 512 {
        let yf = (y as f32) / 512.0;
        for x in 0 .. 512 {
            c.get_mut(x,y).map(|p| {
                let xf = (x as f32) / 512.0;
                p.set_r(xf).set_g(yf).set_b(xf)
            });
        }
    }

    c.save("test.png");
}
