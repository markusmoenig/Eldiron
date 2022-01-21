
use std::fs::File;

enum TileBrand {
    Env,
    EnvBlocking,
    Water
}

struct Tile {
    pos:            (usize, usize),
    brand:          TileBrand,
}

impl Tile {
    fn new(pos: (usize, usize), brand: TileBrand) -> Tile {
        Tile {
            pos,
            brand,
        }
    }
}

// The main TileSet class

pub struct TileSet {
    pub ts1:       Vec<u8>,
    pub ts2:       Vec<u8>,
}

impl TileSet {
    pub fn new() -> TileSet {

        fn load(file_name: &str) -> Vec<u8> {

            let decoder = png::Decoder::new(File::open(file_name).unwrap());
            let mut reader = decoder.read_info().unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            let bytes = &buf[..info.buffer_size()];
    
            bytes.to_vec()
        }

        TileSet {
            ts1: load("assets/ts1b.png"),
            ts2: load("assets/ts2b.png"),
        }
    }
}