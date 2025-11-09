use std::process::Command;
use std::str::FromStr;
use ordered_float::OrderedFloat;
use crate::frame::{Frame, TypedNone};
use super::errors::*;
use super::{Args, Result};
use crate::server::server::Server;
use crate::store::{Value, ZSet};

const MIN_LATITUDE: f64 = -85.05112878;
const MAX_LATITUDE: f64 = 85.05112878;
const MIN_LONGITUDE: f64 = -180.0;
const MAX_LONGITUDE: f64 = 180.0;

const LATITUDE_RANGE: f64 = MAX_LATITUDE - MIN_LATITUDE;
const LONGITUDE_RANGE: f64 = MAX_LONGITUDE - MIN_LONGITUDE;

impl Server {
    /// Adds the specified geospatial items (longitude, latitude, name) to the specified key.
    /// Data is stored into the key as a sorted set, in a way that makes it possible to query
    /// the items with the GEOSEARCH command.
    ///
    /// The command takes arguments in the standard format x,y so the longitude must be specified
    /// before the latitude. There are limits to the coordinates that can be indexed:
    /// areas very near to the poles are not indexable.
    /// ```
    /// GEOADD key [NX | XX] [CH] longitude latitude member [longitude latitude member ...]
    /// ```
    pub async fn geoadd(&mut self, mut args: Args) -> Result {
        let mut store = self.store.lock().await;
        let key = args.pop_front().ok_or(wrong_num_arguments("zadd"))?;
        let set = store
            .kv
            .entry(key)
            .or_insert(Value::ZSet(ZSet::default()))
            .zset_mut()
            .ok_or(wrong_type())?;
        let mut res = 0usize;
        while !args.is_empty() {
            let longitude: f64 = args.pop_front().ok_or(syntax_error())?.parse().map_err(|_| syntax_error())?;
            let latitude: f64 = args.pop_front().ok_or(syntax_error())?.parse().map_err(|_| syntax_error())?;

            if longitude < MIN_LONGITUDE || longitude > MAX_LONGITUDE || latitude < MIN_LATITUDE || latitude > MAX_LATITUDE {
                return Err(format!("ERR invalid longitude,latitude pair {},{}", longitude, latitude).into())
            }

            let key = encode(latitude, longitude);

            let key = OrderedFloat::from(key as f64);
            let value = args.pop_front().ok_or(syntax_error())?;
            if let Some(prev_score) = set.scores.remove(&value) {
                set.ordered.remove(&(prev_score, value.clone()));
            } else {
                res += 1
            }
            set.scores.insert(value.clone(), key);
            set.ordered.insert((key, value));
        }
        Ok(res.into())
    }

    /// Return the positions (longitude,latitude) of all the specified members of the geospatial
    /// index represented by the sorted set at key.
    ///
    /// Given a sorted set representing a geospatial index, populated using the GEOADD command,
    /// it is often useful to obtain back the coordinates of specified members.
    /// When the geospatial index is populated via GEOADD the coordinates are converted into a
    /// 52 bit geohash, so the coordinates returned may not be exactly the ones used in order
    /// to add the elements, but small errors may be introduced.
    //
    /// The command can accept a variable number of arguments so it always returns an array of
    /// positions even when a single element is specified.
    /// ```
    /// GEOPOS key [member [member ...]]
    /// ```
    pub async fn geopos(&mut self, mut args: Args) -> Result {
        let err = || wrong_num_arguments("geopos");

        let store = self.store.lock().await;
        let key = args.pop_front().ok_or(err())?;
        let mut response = vec![];
        if let Some(v) = store.kv.get(&key) {
            let z = v.zset().ok_or(wrong_type())?;
            for member in args {
                let res = if let Some(score) = z.scores.get(&member)
                {
                    let v = score.0 as u64;
                    let pos = decode(v);
                    vec![pos.longitude.to_string(), pos.latitude.to_string()].into()
                } else {
                    Frame::None(TypedNone::Array)
                };
                response.push(res);
            }
        } else {
            for member in args {
                response.push(Frame::None(TypedNone::Array));
            }
        }
        Ok(response.into())
    }

    /// Return the distance between two members in the geospatial index represented by the sorted set.
    ///
    /// Given a sorted set representing a geospatial index, populated using the GEOADD command,
    /// the command returns the distance between the two specified members in the specified unit.
    ///
    /// If one or both the members are missing, the command returns NULL.
    /// ```
    /// GEODIST key member1 member2 [M | KM | FT | MI]
    /// ```
    pub async fn geodist(&mut self, mut args: Args) -> Result {
        let err = || wrong_num_arguments("geopos");

        let store = self.store.lock().await;
        let key = args.pop_front().ok_or(err())?;
        if let Some(v) = store.kv.get(&key) {
            let z = v.zset().ok_or(wrong_type())?;
            let mut coors = vec![];
            for member in args {
                if let Some(score) = z.scores.get(&member)
                {
                    let v = score.0 as u64;
                   coors.push(decode(v));
                }
            }
            if coors.len() < 2 {
                Ok(Frame::None(TypedNone::String))
            } else {
                let first = coors.remove(0);
                let second = coors.remove(0);
                let distance = haversine(&first, &second);
                Ok(distance.to_string().into())
            }
        } else {
            Ok(Frame::None(TypedNone::String))
        }
    }
    
    /// Return the members of a sorted set populated with geospatial information using GEOADD, 
    /// which are within the borders of the area specified by a given shape. 
    /// This command extends the GEORADIUS command, so in addition to searching 
    /// within circular areas, it supports searching within rectangular areas.
    /// ```
    /// GEOSEARCH key <FROMMEMBER member | FROMLONLAT longitude latitude>
    ///   <BYRADIUS radius <M | KM | FT | MI> | BYBOX width height <M | KM |
    ///   FT | MI>> [ASC | DESC] [COUNT count [ANY]] [WITHCOORD] [WITHDIST]
    ///   [WITHHASH]
    /// ```
    pub async fn geosearch(&mut self, mut args: Args) -> Result {
        let err = || wrong_num_arguments("geosearch");

        let store = self.store.lock().await;
        let key = args.pop_front().ok_or(err())?;
        let _fromlatlont = args.pop_front();
        let longitude: f64 = args.pop_front().ok_or(syntax_error())?.parse().map_err(|_| syntax_error())?;
        let latitude: f64 = args.pop_front().ok_or(syntax_error())?.parse().map_err(|_| syntax_error())?;
        let center = Coordinates {longitude, latitude};
        let _byradius = args.pop_front();
        let radius: f64 = args.pop_front().ok_or(syntax_error())?.parse().map_err(|_| syntax_error())?;
        
        if let Some(v) = store.kv.get(&key) {
            let z = v.zset().ok_or(wrong_type())?;
            let res = z.scores.iter().filter_map(|(k, v)| {
                let coord = decode(v.0 as u64);
                let distance = haversine(&center, &coord);
                if distance <= radius {
                    Some(k.clone())
                } else {
                    None
                }
            }).collect::<Vec<_>>();
            Ok(res.into())
        }  else {
            Ok(Frame::None(TypedNone::String))
        }
    }
}

fn spread_int32_to_int64(v: u32) -> u64 {
    let mut result = v as u64;
    result = (result | (result << 16)) & 0x0000FFFF0000FFFF;
    result = (result | (result << 8)) & 0x00FF00FF00FF00FF;
    result = (result | (result << 4)) & 0x0F0F0F0F0F0F0F0F;
    result = (result | (result << 2)) & 0x3333333333333333;
    (result | (result << 1)) & 0x5555555555555555
}

fn interleave(x: u32, y: u32) -> u64 {
    let x_spread = spread_int32_to_int64(x);
    let y_spread = spread_int32_to_int64(y);
    let y_shifted = y_spread << 1;
    x_spread | y_shifted
}

fn encode(latitude: f64, longitude: f64) -> u64 {
    // Normalize to the range 0-2^26
    let normalized_latitude = 2.0_f64.powi(26) * (latitude - MIN_LATITUDE) / LATITUDE_RANGE;
    let normalized_longitude = 2.0_f64.powi(26) * (longitude - MIN_LONGITUDE) / LONGITUDE_RANGE;

    // Truncate to integers
    let lat_int = normalized_latitude as u32;
    let lon_int = normalized_longitude as u32;

    interleave(lat_int, lon_int)
}

#[derive(Debug)]
struct Coordinates {
    latitude: f64,
    longitude: f64,
}

fn compact_int64_to_int32(v: u64) -> u32 {
    let mut result = v & 0x5555555555555555;
    result = (result | (result >> 1)) & 0x3333333333333333;
    result = (result | (result >> 2)) & 0x0F0F0F0F0F0F0F0F;
    result = (result | (result >> 4)) & 0x00FF00FF00FF00FF;
    result = (result | (result >> 8)) & 0x0000FFFF0000FFFF;
    ((result | (result >> 16)) & 0x00000000FFFFFFFF) as u32  // Cast to u32
}

fn convert_grid_numbers_to_coordinates(grid_latitude_number: u32, grid_longitude_number: u32) -> Coordinates {
    // Calculate the grid boundaries
    let grid_latitude_min = MIN_LATITUDE + LATITUDE_RANGE * (grid_latitude_number as f64 / 2.0_f64.powi(26));
    let grid_latitude_max = MIN_LATITUDE + LATITUDE_RANGE * ((grid_latitude_number + 1) as f64 / 2.0_f64.powi(26));
    let grid_longitude_min = MIN_LONGITUDE + LONGITUDE_RANGE * (grid_longitude_number as f64 / 2.0_f64.powi(26));
    let grid_longitude_max = MIN_LONGITUDE + LONGITUDE_RANGE * ((grid_longitude_number + 1) as f64 / 2.0_f64.powi(26));

    // Calculate the center point of the grid cell
    let latitude = (grid_latitude_min + grid_latitude_max) / 2.0;
    let longitude = (grid_longitude_min + grid_longitude_max) / 2.0;

    Coordinates { latitude, longitude }
}

fn decode(geo_code: u64) -> Coordinates {
    // Align bits of both latitude and longitude to take even-numbered position
    let y = geo_code >> 1;
    let x = geo_code;

    // Compact bits back to 32-bit ints
    let grid_latitude_number = compact_int64_to_int32(x);
    let grid_longitude_number = compact_int64_to_int32(y);

    convert_grid_numbers_to_coordinates(grid_latitude_number, grid_longitude_number)
}

fn haversine(origin: &Coordinates, destination: &Coordinates) -> f64 {
    const R: f64 = 6372797.560856;

    let lat1 = origin.latitude.to_radians();
    let lat2 = destination.latitude.to_radians();
    let d_lat = lat2 - lat1;
    let d_lon = (destination.longitude - origin.longitude).to_radians();

    let a = (d_lat / 2.0).sin().powi(2) + (d_lon / 2.0).sin().powi(2) * lat1.cos() * lat2.cos();
    let c = 2.0 * a.sqrt().asin();
    R * c
}