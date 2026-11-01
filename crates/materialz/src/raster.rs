//! Bounded embedded raster samples used by host-imported image materials.
//! Pixel data lives in the immutable GPU parameter buffer, never shader code.
use crate::{Error, Result};
#[derive(Clone, Debug)]
pub(crate) struct Raster {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[f32; 4]>,
    pub frames: usize,
    pub address: usize,
    pub filter: usize,
    pub centered: bool,
}
impl Raster {
    pub fn parse(source: &str) -> Result<Option<Self>> {
        let mut rows = source
            .lines()
            .filter_map(|line| line.trim().strip_prefix("# raster-rgba8 "));
        let Some(line) = rows.next() else {
            return Ok(None);
        };
        if rows.next().is_some() {
            return Err(Error("Only one embedded raster per tile is allowed".into()));
        }
        let mut parts = line.split_whitespace();
        let width = parts
            .next()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let height = parts
            .next()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let space = parts.next().unwrap_or("");
        let hex = parts.next().unwrap_or("");
        let count = width
            .checked_mul(height)
            .filter(|n| *n > 0 && *n <= 1_048_576)
            .ok_or_else(|| {
                Error("Raster dimensions must contain between 1 and 1048576 pixels".into())
            })?;
        if !matches!(space, "srgb" | "linear")
            || hex.len() != count * 8
            || parts.next().is_some()
            || !hex.is_ascii()
        {
            return Err(Error(
                "Invalid raster dimensions, color space or RGBA8 payload".into(),
            ));
        }
        let mut pixels = Vec::with_capacity(count);
        for pixel in hex.as_bytes().chunks_exact(8) {
            let mut value = [0.; 4];
            for c in 0..4 {
                let byte = std::str::from_utf8(&pixel[c * 2..c * 2 + 2])
                    .ok()
                    .and_then(|v| u8::from_str_radix(v, 16).ok())
                    .ok_or_else(|| Error("Invalid raster hex data".into()))?;
                let x = byte as f32 / 255.;
                value[c] = if space == "srgb" && c < 3 {
                    if x <= 0.04045 {
                        x / 12.92
                    } else {
                        ((x + 0.055) / 1.055).powf(2.4)
                    }
                } else {
                    x
                };
            }
            pixels.push(value);
        }
        Ok(Some(Self {
            width,
            height,
            pixels,
            frames: 1,
            address: 2,
            filter: 0,
            centered: false,
        }))
    }
    pub fn from_image(
        image: Option<&crate::image_source::ImageSource>,
        address: usize,
        filter: usize,
        channel: usize,
        centered: bool,
    ) -> Result<Self> {
        if address > 2 || filter > 1 || channel > 4 {
            return Err(Error("Invalid image sampling option".into()));
        }
        let Some(image) = image else {
            return Ok(Self {
                width: 1,
                height: 1,
                pixels: vec![[0.; 4]],
                frames: 1,
                address,
                filter,
                centered,
            });
        };
        image.validate()?;
        let frames = image.frame_count();
        let count = image.width * image.height;
        let channel = ["color", "roughness", "metallic", "opacity", "emission"][channel];
        let decode = |name: &str, frame: usize| -> Result<Option<Vec<[f32; 4]>>> {
            let Some(plane) = image.planes.iter().find(|p| p.channel == name) else {
                return Ok(None);
            };
            let hex = &plane.frames[frame % plane.frames.len()];
            Ok(Some(
                Self::parse(&format!(
                    "# raster-rgba8 {} {} {} {}",
                    image.width, image.height, plane.space, hex
                ))?
                .unwrap()
                .pixels,
            ))
        };
        let mut pixels = Vec::with_capacity(count * frames);
        for frame in 0..frames {
            let color = decode("color", frame)?.unwrap();
            let opacity = decode("opacity", frame)?;
            let plane = if channel == "color" {
                None
            } else {
                decode(channel, frame)?
            };
            for i in 0..count {
                let alpha = color[i][3] * opacity.as_ref().map_or(1., |p| p[i][0]);
                let mut pixel = match channel {
                    "color" => color[i],
                    "opacity" => [alpha, alpha, alpha, 1.],
                    "roughness" => plane.as_ref().map_or([0.5, 0.5, 0.5, 1.], |p| p[i]),
                    _ => plane.as_ref().map_or([0., 0., 0., 1.], |p| p[i]),
                };
                if channel == "color" {
                    pixel[3] = alpha;
                }
                pixels.push(pixel);
            }
        }
        Ok(Self {
            width: image.width,
            height: image.height,
            pixels,
            frames,
            address,
            filter,
            centered,
        })
    }
    fn pixel(&self, x: i32, y: i32, frame: usize) -> [f32; 4] {
        let (w, h) = (self.width as i32, self.height as i32);
        if self.address == 0 && (x < 0 || y < 0 || x >= w || y >= h) {
            return [0.; 4];
        }
        let index = |v: i32, n: i32| {
            if self.address == 2 {
                v.rem_euclid(n)
            } else {
                v.clamp(0, n - 1)
            }
        };
        self.pixels[frame * self.width * self.height
            + index(y, h) as usize * self.width
            + index(x, w) as usize]
    }
    pub fn sample(&self, uv: [f32; 2], time: f32) -> [f32; 4] {
        let mut p = uv.map(|v| v + if self.centered { 0.5 } else { 0. });
        if self.address == 0 && p.iter().any(|v| *v < 0. || *v >= 1.) {
            return [0.; 4];
        }
        for v in &mut p {
            *v = if self.address == 2 {
                *v - v.floor()
            } else {
                v.clamp(0., 1.)
            };
        }
        let frame =
            (((time - time.floor()) * self.frames as f32).floor() as usize).min(self.frames - 1);
        if self.filter == 0 {
            return self.pixel(
                (p[0] * self.width as f32).floor() as i32,
                (p[1] * self.height as f32).floor() as i32,
                frame,
            );
        }
        let p = [
            p[0] * self.width as f32 - 0.5,
            p[1] * self.height as f32 - 0.5,
        ];
        let (x, y) = (p[0].floor() as i32, p[1].floor() as i32);
        let (fx, fy) = (p[0] - p[0].floor(), p[1] - p[1].floor());
        let mut value = [0.; 4];
        for (dx, dy, w) in [
            (0, 0, (1. - fx) * (1. - fy)),
            (1, 0, fx * (1. - fy)),
            (0, 1, (1. - fx) * fy),
            (1, 1, fx * fy),
        ] {
            let pixel = self.pixel(x + dx, y + dy, frame);
            for c in 0..3 {
                value[c] += pixel[c] * pixel[3] * w;
            }
            value[3] += pixel[3] * w;
        }
        if value[3] > 0. {
            for c in 0..3 {
                value[c] /= value[3];
            }
        }
        value
    }
    pub fn metal_helpers(&self, id: usize, buffered: bool, offset: usize) -> String {
        let arg = if buffered {
            ", constant float4* params"
        } else {
            ""
        };
        let load = if buffered {
            format!("params[{offset}u+index]")
        } else {
            format!("mz_bitmap_{id}[index]")
        };
        let (w, h) = (self.width, self.height);
        let address = match self.address {
            0 => format!("if(x<0 || y<0 || x>={w} || y>={h}) return float4(0);"),
            2 => format!("x=((x%{w})+{w})%{w};y=((y%{h})+{h})%{h};"),
            _ => format!(
                "x=clamp(x,0,{last_x});y=clamp(y,0,{last_y});",
                last_x = w - 1,
                last_y = h - 1
            ),
        };
        format!(
            "float4 mz_image_pixel_{id}(int x,int y,uint frame{arg}) {{{address} uint index=frame*{count}u+uint(y)*{w}u+uint(x);return {load};}}\n",
            count = w * h
        )
    }
    pub fn metal_sample(&self, id: usize, buffered: bool) -> String {
        use std::fmt::Write;
        let arg = if buffered { ", params" } else { "" };
        let center = if self.centered { "+float2(0.5f)" } else { "" };
        let mut s = format!("float2 p=uv{center};\n");
        if self.address == 0 {
            s.push_str("if(p.x<0.0f || p.y<0.0f || p.x>=1.0f || p.y>=1.0f) {MzSample empty=mz_image(float3(0));empty.color=float4(0);return empty;}\n");
        }
        s.push_str(if self.address == 2 {
            "p=fract(p);\n"
        } else {
            "p=clamp(p,float2(0),float2(1));\n"
        });
        writeln!(
            s,
            "uint frame=min(uint(floor(fract(ctx.time)*{}.0f)),{}u);",
            self.frames,
            self.frames - 1
        )
        .unwrap();
        if self.filter == 0 {
            writeln!(s,"float4 mz_pixel=mz_image_pixel_{id}(int(floor(p.x*{}.0f)),int(floor(p.y*{}.0f)),frame{arg});",self.width,self.height).unwrap();
        } else {
            writeln!(s,"p=p*float2({}.0f,{}.0f)-float2(0.5f);int x=int(floor(p.x));int y=int(floor(p.y));float2 f=fract(p);float4 mz_pixel=float4(0);",self.width,self.height).unwrap();
            for (i, (dx, dy, weight)) in [
                (0, 0, "(1.0f-f.x)*(1.0f-f.y)"),
                (1, 0, "f.x*(1.0f-f.y)"),
                (0, 1, "(1.0f-f.x)*f.y"),
                (1, 1, "f.x*f.y"),
            ]
            .iter()
            .enumerate()
            {
                writeln!(s,"float4 p{i}=mz_image_pixel_{id}(x+{dx},y+{dy},frame{arg});mz_pixel+=float4(p{i}.rgb*p{i}.a,p{i}.a)*({weight});").unwrap();
            }
            s.push_str("mz_pixel.rgb/=max(mz_pixel.a,0.000001f);\n");
        }
        s.push_str(
            "MzSample result=mz_image(mz_pixel.rgb);result.color=mz_pixel;return result;\n}\n",
        );
        s
    }
}
