use glam::*;
use noise::{NoiseFn, Worley};

pub struct DistanceField
{
    lucifer: Worley,
}

impl DistanceField
{

    pub fn new() -> Self
    {
        Self
        {
            lucifer: Default::default(),
        }
    }

    pub fn gen(&self, pos: DVec3) -> u8
    {
        // let df = df_sphere(pos, 5.0);
        // let df = df_plane(pos, dvec3(1.0, 1.0, 1.0), 1.0);
       // let df = df + 2.0 * pos.x.sin() * pos.y.sin() * pos.z.sin();
        let df = self.lucifer.get([pos.x, pos.y, pos.z]);
        Self::compress(df)
    }

    pub fn compress_range(vals: Vec<f64>) -> Vec<u8> {
        let mut ret = vec![0; vals.len()];
        for i in 0..vals.len() {
            let d  = (vals[i] * 64.0) as i32 + 128;
            ret[i] = std::cmp::min(std::cmp::max(0, d), 255) as u8;
        }
        ret
    }

    pub fn compress(v: f64) -> u8 {
        let d  = (v * 64.0) as i32 + 128;
        std::cmp::min(std::cmp::max(0, d), 255) as u8
    }

}

