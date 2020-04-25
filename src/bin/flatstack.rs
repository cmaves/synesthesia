use clap::{App, Arg,ArgMatches};
use ecp::channel;
use ecp::color::Color;
use ecp::controller::{rs_ws281x, Renderer};
use ham;
use jack;
use std::num::{NonZeroU8,NonZeroU16};
use std::str::FromStr;
use std::time::{Duration,Instant};
use std::thread::{Builder,sleep};
use synesthesia;
use synesthesia::audio::InactiveAudioSource;
use synesthesia::control::{AudioVisualizer, Algorithm, Effect};

enum Src {
    Jack(jack::Client),
    Pulse,
}
enum Mode {
    Local,
    Err,
}
pub fn main() {
    #[cfg(any(feature = "ecp", feature="rpi"))]
    eprintln!("feature enabled");
    let parser = parser();
    let args = parser.get_matches();
    match args.value_of("source").unwrap() {
        //"jack" =>  jack::Client::new(args.value_of("flatstack").unwrap(), jack::ClientOptions::NO_START_SERVER).unwrap().0),
        "jack" => {
            let src = jack::Client::new(
                args.value_of("clientname").unwrap(),
                jack::ClientOptions::NO_START_SERVER,
            )
            .unwrap()
            .0;
            start_sender(args, src)
        }
        _ => unimplemented!(),
    }
}
pub fn start_sender<T: InactiveAudioSource>(args: ArgMatches, src: T) {
    match args.value_of("mode").unwrap() {
        "local" => {
            let (sender, recv) = channel(2);
            let pin = u8::from_str(args.value_of("led_pin").unwrap()).unwrap() as i32;
            let count = u16::from_str(args.value_of("led_count").unwrap()).unwrap() as i32;
            Builder::new().name("rendering".to_string()).spawn(move || {
                let channel = rs_ws281x::ChannelBuilder::new()
                    .pin(pin)
                    .strip_type(rs_ws281x::StripType::Ws2812)
                    .count(count)
                    .brightness(255)
                    .build();
                let ctl = rs_ws281x::ControllerBuilder::new()
                    .freq(800_000)
                    .channel(0, channel)
                    .build()
                    .unwrap();
                let start = Instant::now(); 
                let mut renderer =Renderer::new(recv, ctl);
                renderer.blend = 3;
                renderer.verbose = true;
                renderer.color_map[2] = Color::YELLOW;
                renderer.color_map[3] = Color::GREEN;
                renderer.color_map[4] = Color::BLUE;
                panic!("Rendering thread quit: {:?}", renderer.update_leds_loop(60.0));
            });
            let mut av = AudioVisualizer::new(src, Effect::Stereo4FlatStack(Algorithm::Quadratic, false)).unwrap();
            av.senders.push(Box::new(sender));
            av.process_loop();
        }
        _ => unimplemented!(),
    }
}

fn parser<'a, 'b>() -> App<'a, 'b> {
    App::new("Flat Stack")
        .version("0.1")
        .author("Curtis Maves <curtismaves@gmail.com")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("src")
                .value_name("SOURCE")
                .possible_value("jack")
                .help("Sets the audio source")
                .takes_value(true)
                .default_value("jack"),
        )
        .arg(
            Arg::with_name("value")
                .short("a")
                .long("alg")
                .value_name("ALGORITHM")
                .possible_values(&["linear", "quadratic"])
                .help("Sets the algorithm used to scale the light bars.")
                .default_value("quadratic")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("brightness")
                .short("b")
                .long("brightness")
                .value_name("BRIGHTNESS")
                .takes_value(true)
                .validator(|s| {
                    NonZeroU8::from_str(&s)
                        .map(|_| ())
                        .map_err(|e| format!("{:?}", e))
                }),
        )
        .arg(
            Arg::with_name("clientname")
                .long("clientname")
                .short("n")
                .value_name("NAME")
                .default_value("flatstack")
                .help("Sets the name to be used by the audio client")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mode")
                .short("m")
                .long("mode")
                .value_name("MODE")
                .possible_values(&["local", "ham"])
                .default_value("local"),
        )
        .arg(Arg::with_name("led_pin")
            .short("p")
            .long("pin")
            .value_name("PIN")
            .takes_value(true)
            .validator(|s| 
                u8::from_str(&s).map(|_| ()).map_err(|e| format!("{:?}", e))
            )
            .default_value("18")
            )
        .arg(Arg::with_name("led_count")
            .short("c")
            .long("count")
            .value_name("COUNT")
            .takes_value(true)
            .validator(|s|  NonZeroU16::from_str(&s).map(|_| ()).map_err(|e| format!("{:?}", e)))
            .default_value("288")
            )

}
