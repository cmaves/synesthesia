use clap::{App, Arg, ArgMatches};
use ecp::color::Color;
use ecp::controller::{rs_ws281x, Renderer};
use ecp::{channel, Sender};
use gpio_cdev::Chip;
use ham::rfm69::Rfm69;
use ham::IntoPacketSender;
use jack;
use spidev::Spidev;
use std::num::{NonZeroU16, NonZeroU8};
use std::str::FromStr;
use std::thread::Builder;
use std::time::{Duration, Instant};
use synesthesia;
use synesthesia::audio::InactiveAudioSource;
use synesthesia::control::{Algorithm, AudioVisualizer, Effect};

enum Src {
    Jack(jack::Client),
    Pulse,
}
enum Mode {
    Local,
    Err,
}
pub fn main() {
    #[cfg(any(feature = "ecp", feature = "rpi"))]
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
fn start_sender<T: InactiveAudioSource>(args: ArgMatches, src: T) {
    let verbose = args.occurrences_of("verbose") as u8;
    match args.value_of("mode").unwrap() {
        "local" => {
            let (sender, recv) = channel(2);
            let pin = u8::from_str(args.value_of("led_pin").unwrap()).unwrap() as i32;
            let count = u16::from_str(args.value_of("led_count").unwrap()).unwrap() as i32;
            let brightness =
                u8::from_str(args.value_of("brightness").unwrap()).unwrap() as f32 / 255.0;
            Builder::new()
                .name("rendering".to_string())
                .spawn(move || {
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
                    let mut renderer = Renderer::new(recv, ctl);
                    renderer.blend = 3;
                    renderer.verbose = verbose;
                    renderer.color_map[2] = Color::YELLOW;
                    renderer.color_map[3] = Color::GREEN;
                    renderer.color_map[4] = Color::BLUE;
                    for color in renderer.color_map[0..5].iter_mut() {
                        *color *= brightness;
                    }
                    panic!(
                        "Rendering thread quit: {:?}",
                        renderer.update_leds_loop(60.0)
                    );
                })
                .unwrap();
            start_av(verbose, src, sender);
        }
        "ham" => {
            let mut chip = Chip::new("/dev/gpiochip0").unwrap();
            let en = chip
                .get_line(u32::from_str(args.value_of("en").unwrap()).unwrap())
                .unwrap();
            let rst = chip
                .get_line(u32::from_str(args.value_of("rst").unwrap()).unwrap())
                .unwrap();
            let spi = Spidev::open(args.value_of("spi").unwrap()).unwrap();
            let power = i8::from_str(args.value_of("power").unwrap()).unwrap();
            let bitrate = u32::from_str(args.value_of("bitrate").unwrap()).unwrap();
            let mut rfm = Rfm69::new(rst, en, spi).unwrap();
            rfm.set_bitrate(bitrate).unwrap();
            rfm.set_power(power).unwrap();
            let mut sender = rfm.into_packet_sender(1).unwrap();
            sender.set_verbose(verbose).unwrap();
            start_av(verbose, src, sender);
        }
        _ => unimplemented!(),
    }
}

fn start_av<S: InactiveAudioSource, T: Sender + 'static>(verbose: u8, src: S, sender: T) {
    let mut av =
        AudioVisualizer::new(src, Effect::Stereo4FlatStack(Algorithm::Quadratic, false)).unwrap();
    av.senders.push(Box::new(sender));
    av.verbose = verbose;
    av.process_loop();
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
        .arg(
            Arg::with_name("led_pin")
                .short("p")
                .long("pin")
                .value_name("PIN")
                .takes_value(true)
                .validator(|s| u8::from_str(&s).map(|_| ()).map_err(|e| format!("{:?}", e)))
                .default_value("18"),
        )
        .arg(
            Arg::with_name("led_count")
                .short("c")
                .long("count")
                .value_name("COUNT")
                .takes_value(true)
                .validator(|s| {
                    NonZeroU16::from_str(&s)
                        .map(|_| ())
                        .map_err(|e| format!("{:?}", e))
                })
                .default_value("288"),
        )
        .arg(
            Arg::with_name("spi")
                .short("i")
                .long("spi")
                .value_name("SPIPATH")
                .takes_value(true)
                .default_value("/dev/spidev0.0"),
        )
        .arg(
            Arg::with_name("rst")
                .short("r")
                .long("reset")
                .value_name("RSTPIN")
                .takes_value(true)
                .validator(|s| {
                    NonZeroU8::from_str(&s)
                        .map(|_| ())
                        .map_err(|e| format!("{:?}", e))
                })
                .default_value("24"),
        )
        .arg(
            Arg::with_name("en")
                .short("e")
                .long("enable")
                .value_name("ENPIN")
                .takes_value(true)
                .validator(|s| {
                    NonZeroU8::from_str(&s)
                        .map(|_| ())
                        .map_err(|e| format!("{:?}", e))
                })
                .default_value("3"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true),
        )
        .arg(
            Arg::with_name("power")
                .short("o")
                .long("power")
                .value_name("LEVEL")
                .takes_value(true)
                .validator(|s| {
                    let power = i8::from_str(&s).map_err(|e| format!("{:?}", e))?;
                    if -18 <= power && power <= 20 {
                        Ok(())
                    } else {
                        Err("Power must be between [-18,20].".to_string())
                    }
                })
                .default_value("13")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::with_name("bitrate")
                .long("bitrate")
                .value_name("BPS")
                .takes_value(true)
                .validator(|s| {
                    let rate = u32::from_str(&s).map_err(|e| format!("{:?}", e))?;
                    if rate <= 300_000 {
                        Ok(())
                    } else {
                        Err("Rate cannot be greater than 300_000 bps.".to_string())
                    }
                })
                .default_value("4800"),
        )
}
