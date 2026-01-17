use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use crc32fast::Hasher;
use flate2::write::ZlibEncoder;
use flate2::Compression;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

const CICP_PRIMARIES_DISPLAY_P3: u8 = 12;
const CICP_TRANSFER_SRGB: u8 = 13;
const CICP_MATRIX_IDENTITY: u8 = 0;
const CICP_FULL_RANGE: u8 = 1;

const AFTER_HELP: &str =
    "Default output file: oklch(l c h).png or oklch(l c h \u{2215} a).png (L normalized to 0..1).";

#[derive(Parser, Debug)]
#[command(
    name = "oklch-pixel",
    version,
    about = "Generate a 1x1 PNG in Display P3 from OKLCH.",
    after_help = AFTER_HELP,
    subcommand_negates_reqs = true,
    args_conflicts_with_subcommands = true,
    subcommand_precedence_over_arg = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(
        long,
        value_enum,
        default_value_t = BitDepth::Eight,
        help = "Output bit depth"
    )]
    bit_depth: BitDepth,

    #[arg(long, value_name = "path", help = "Explicit output file path")]
    output_file: Option<String>,

    #[arg(
        value_name = "L",
        help = "Lightness: 0..1 or percent (e.g. 62.5%)."
    )]
    l: String,

    #[arg(
        value_name = "C",
        help = "Chroma (≥ 0)."
    )]
    c: String,

    #[arg(
        value_name = "H",
        help = "Hue in degrees."
    )]
    h: String,

    #[arg(value_name = "A", help = "Alpha 0..1 (optional). If provided, output is RGBA.")]
    a: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Generate shell completions")]
    GenerateCompletions {
        #[arg(value_enum, value_name = "shell")]
        shell: CompletionShell,
    },
}

#[derive(Parser, Debug)]
#[command(name = "oklch-pixel")]
struct CompletionCli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum BitDepth {
    #[value(name = "8")]
    Eight,
    #[value(name = "16")]
    Sixteen,
}

impl BitDepth {
    fn as_u8(self) -> u8 {
        match self {
            BitDepth::Eight => 8,
            BitDepth::Sixteen => 16,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum CompletionShell {
    #[value(name = "bash")]
    Bash,
    #[value(name = "elvish")]
    Elvish,
    #[value(name = "fish")]
    Fish,
    #[value(name = "powershell", alias = "power-shell")]
    PowerShell,
    #[value(name = "zsh")]
    Zsh,
}

impl CompletionShell {
    fn as_shell(self) -> Shell {
        match self {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Elvish => Shell::Elvish,
            CompletionShell::Fish => Shell::Fish,
            CompletionShell::PowerShell => Shell::PowerShell,
            CompletionShell::Zsh => Shell::Zsh,
        }
    }
}

#[derive(Clone, Copy)]
struct Pixel {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("generate-completions") {
        let completion_cli = CompletionCli::parse_from(&args);
        let Commands::GenerateCompletions { shell } = completion_cli.command;
        let mut cmd = Cli::command();
        let bin_name = cmd.get_name().to_string();
        generate(shell.as_shell(), &mut cmd, bin_name, &mut io::stdout());
        return;
    }

    let cli = Cli::parse_from(&args);

    let l_str = cli.l;
    let c_str = cli.c;
    let h_str = cli.h;
    let a_str = cli.a;

    let include_alpha = a_str.is_some();
    let l = parse_l(&l_str).unwrap_or_else(|e| fail(&e));
    let c = parse_non_negative(&c_str, "C").unwrap_or_else(|e| fail(&e));
    let h = parse_f64(&h_str, "H").unwrap_or_else(|e| fail(&e));
    let alpha = match a_str {
        Some(value) => parse_unit_range(&value, "A").unwrap_or_else(|e| fail(&e)),
        None => 1.0,
    };

    let output = cli
        .output_file
        .unwrap_or_else(|| default_output_name(l, c, h, include_alpha.then_some(alpha)));
    let bit_depth = cli.bit_depth.as_u8();

    let (r_lin, g_lin, b_lin, clipped) = oklch_to_display_p3_linear(l, c, h)
        .unwrap_or_else(|e| fail(&e));
    if clipped {
        eprintln!("warning: color out of Display P3 gamut; clipped");
    }

    let pixel = Pixel {
        r: srgb_encode(r_lin),
        g: srgb_encode(g_lin),
        b: srgb_encode(b_lin),
        a: alpha,
    };

    if let Err(err) = write_png(Path::new(&output), bit_depth, include_alpha, pixel) {
        fail(&format!("failed to write PNG: {err}"));
    }
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    eprintln!("Run with --help for usage.");
    process::exit(1);
}

fn default_output_name(l: f64, c: f64, h: f64, a: Option<f64>) -> String {
    let l_str = format_component(l);
    let c_str = format_component(c);
    let h_str = format_component(h);
    if let Some(alpha) = a {
        let a_str = format_component(alpha);
        format!(
            "oklch({} {} {} \u{2215} {}).png",
            l_str, c_str, h_str, a_str
        )
    } else {
        format!("oklch({} {} {}).png", l_str, c_str, h_str)
    }
}

fn format_component(value: f64) -> String {
    let mut s = format!("{value}");
    if s == "-0" || s == "-0.0" {
        s = "0".to_string();
    }
    s
}

fn parse_l(input: &str) -> Result<f64, String> {
    if let Some(value) = input.strip_suffix('%') {
        let parsed = parse_f64(value, "L%")?;
        if !(0.0..=100.0).contains(&parsed) {
            return Err("L% must be between 0 and 100".to_string());
        }
        Ok(parsed / 100.0)
    } else {
        let parsed = parse_f64(input, "L")?;
        if !(0.0..=1.0).contains(&parsed) {
            return Err("L must be between 0 and 1 (or use %)".to_string());
        }
        Ok(parsed)
    }
}

fn parse_non_negative(input: &str, name: &str) -> Result<f64, String> {
    let value = parse_f64(input, name)?;
    if value < 0.0 {
        return Err(format!("{name} must be >= 0"));
    }
    Ok(value)
}

fn parse_unit_range(input: &str, name: &str) -> Result<f64, String> {
    let value = parse_f64(input, name)?;
    if !(0.0..=1.0).contains(&value) {
        return Err(format!("{name} must be between 0 and 1"));
    }
    Ok(value)
}

fn parse_f64(input: &str, name: &str) -> Result<f64, String> {
    let value = input
        .parse::<f64>()
        .map_err(|_| format!("{name} must be a number"))?;
    if !value.is_finite() {
        return Err(format!("{name} must be finite"));
    }
    Ok(value)
}

fn oklch_to_display_p3_linear(l: f64, c: f64, h_deg: f64) -> Result<(f64, f64, f64, bool), String> {
    let h = h_deg.rem_euclid(360.0).to_radians();
    let a = c * h.cos();
    let b = c * h.sin();

    let (x, y, z) = oklab_to_xyz(l, a, b);
    let (r, g, b) = xyz_to_lin_display_p3(x, y, z);

    if !r.is_finite() || !g.is_finite() || !b.is_finite() {
        return Err("color conversion produced a non-finite value".to_string());
    }

    let mut clipped = false;
    let r = clamp01(r, &mut clipped);
    let g = clamp01(g, &mut clipped);
    let b = clamp01(b, &mut clipped);

    Ok((r, g, b, clipped))
}

fn oklab_to_xyz(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    // Oklab is defined over linear sRGB; convert LMS to linear sRGB, then to XYZ D65.
    let r_lin = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g_lin = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b_lin = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    let x = 0.4124564 * r_lin + 0.3575761 * g_lin + 0.1804375 * b_lin;
    let y = 0.2126729 * r_lin + 0.7151522 * g_lin + 0.0721750 * b_lin;
    let z = 0.0193339 * r_lin + 0.1191920 * g_lin + 0.9503041 * b_lin;

    (x, y, z)
}

fn xyz_to_lin_display_p3(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let r = 2.493496911941425 * x - 0.9313836179191239 * y - 0.40271078445071684 * z;
    let g = -0.8294889695615747 * x + 1.7626640603183463 * y + 0.023624685841943577 * z;
    let b = 0.03584583024378447 * x - 0.07617238926804182 * y + 0.9568845240076872 * z;
    (r, g, b)
}

fn clamp01(value: f64, clipped: &mut bool) -> f64 {
    if value < 0.0 {
        *clipped = true;
        0.0
    } else if value > 1.0 {
        *clipped = true;
        1.0
    } else {
        value
    }
}

fn srgb_encode(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

fn write_png(
    path: &Path,
    bit_depth: u8,
    include_alpha: bool,
    pixel: Pixel,
) -> io::Result<()> {
    let mut file = File::create(path)?;

    file.write_all(&PNG_SIGNATURE)?;

    let color_type = if include_alpha { 6 } else { 2 };
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.push(bit_depth);
    ihdr.push(color_type);
    ihdr.push(0);
    ihdr.push(0);
    ihdr.push(0);
    write_chunk(&mut file, b"IHDR", &ihdr)?;

    let cicp = [
        CICP_PRIMARIES_DISPLAY_P3,
        CICP_TRANSFER_SRGB,
        CICP_MATRIX_IDENTITY,
        CICP_FULL_RANGE,
    ];
    write_chunk(&mut file, b"cICP", &cicp)?;

    let mut raw = Vec::new();
    raw.push(0);
    push_sample(&mut raw, pixel.r, bit_depth);
    push_sample(&mut raw, pixel.g, bit_depth);
    push_sample(&mut raw, pixel.b, bit_depth);
    if include_alpha {
        push_sample(&mut raw, pixel.a, bit_depth);
    }

    let compressed = zlib_compress(&raw)?;
    write_chunk(&mut file, b"IDAT", &compressed)?;
    write_chunk(&mut file, b"IEND", &[])?;

    Ok(())
}

fn zlib_compress(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}

fn push_sample(buf: &mut Vec<u8>, value: f64, bit_depth: u8) {
    let clamped = value.clamp(0.0, 1.0);
    match bit_depth {
        8 => buf.push((clamped * 255.0).round() as u8),
        16 => {
            let sample = (clamped * 65535.0).round() as u16;
            buf.extend_from_slice(&sample.to_be_bytes());
        }
        _ => {}
    }
}

fn write_chunk<W: Write>(writer: &mut W, chunk_type: &[u8; 4], data: &[u8]) -> io::Result<()> {
    let length = u32::try_from(data.len()).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "chunk too large")
    })?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(chunk_type)?;
    writer.write_all(data)?;

    let mut hasher = Hasher::new();
    hasher.update(chunk_type);
    hasher.update(data);
    let crc = hasher.finalize();
    writer.write_all(&crc.to_be_bytes())?;
    Ok(())
}
