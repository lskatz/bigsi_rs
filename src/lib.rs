//simple bigsi implementation
// methods:
// 1. set; set specific bit at specific position in the vector
// 2. get: get all accessions (bitvector positions) given a number of hashes
extern crate bv;

#[macro_use]
extern crate serde_derive;
use bincode::{deserialize_from, serialize};
use bv::BitVec;
use bv::BitsExt;
use bv::*;
use fasthash;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Bigsi {
    pub bigsi: Vec<BitVec>, // vector of bitvecs
    pub num_hashes: u64,    // # of hashes needed
    pub accessions: u64,
}

//m: bigsi length, n: number of accessions, eta: num_hashes
impl Bigsi {
    pub fn new(m: usize, n: u64, eta: u64) -> Bigsi {
        Bigsi {
            bigsi: vec![BitVec::new_fill(false, n); m],
            num_hashes: eta,
            accessions: n,
        }
    }
    pub fn default() -> Bigsi {
        Bigsi {
            bigsi: vec![BitVec::new_fill(false, 10); 1000],
            num_hashes: 2,
            accessions: 10,
        }
    }
    pub fn insert(&mut self, accession: u64, value: &str) {
        // Generate a bit index for each of the hash functions needed
        for i in 0..self.num_hashes {
            let bit_index = (fasthash::xx::hash64_with_seed(&value.as_bytes(), i as u64)
                % self.bigsi.len() as u64) as usize;
            self.bigsi[bit_index].set(accession, true);
        }
    }
    //set all bit_vecs that have all false bits to BitVec::new()
    pub fn slim(&mut self) {
        let empty = BitVec::new_fill(false, self.accessions);
        let mut empties = Vec::new();
        for (i, v) in self.bigsi.iter().enumerate() {
            if v == &empty {
                empties.push(i);
            }
        }
        for i in empties {
            self.bigsi[i] = BitVec::new();
        }
    }
    // given a value, return a vector with accessions containing the query value
    pub fn get(&self, value: &str) -> Vec<usize> {
        let mut final_vec = BitVec::new_fill(true, self.accessions as u64);
        let mut hits = Vec::new();
        for i in 0..self.num_hashes {
            let bit_index = (fasthash::xx::hash64_with_seed(&value.as_bytes(), i as u64)
                % (self.bigsi.len() as u64)) as usize;
            if self.bigsi[bit_index].is_empty() {
                return hits;
            } else {
                final_vec = final_vec.bit_and(&self.bigsi[bit_index]).to_bit_vec();
            }
        }
        for item in 0..self.accessions {
            if final_vec[item] {
                hits.push(item as usize);
            }
        }
        hits
    }
    //return hits as bit vector
    pub fn get_bv(&self, value: &str) -> BitVec {
        let mut final_vec = BitVec::new_fill(true, self.accessions as u64);
        for i in 0..self.num_hashes {
            let bit_index = (fasthash::xx::hash64_with_seed(&value.as_bytes(), i as u64)
                % (self.bigsi.len() as u64)) as usize;
            if self.bigsi[bit_index].is_empty() {
                return self.bigsi[bit_index].to_owned();
            } else {
                final_vec = final_vec.bit_and(&self.bigsi[bit_index]).to_bit_vec();
            }
        }
        final_vec
    }
    //merge two indices
    pub fn merge(&mut self, other_bigsi: &Bigsi) {
        //assert critical parameters are the same
        if (self.num_hashes != other_bigsi.num_hashes)
            || (self.bigsi.len() != other_bigsi.bigsi.len())
        {
            panic!("indices do not use the same parameters!");
        };
        //have to take 0 length bitvectors into account!
        self.bigsi = self
            .bigsi
            .iter()
            .zip(other_bigsi.bigsi.iter())
            .map(|(x, y)| {
                if (x.len() == 0) && (y.len() > 0) {
                    BitVec::new_fill(false, self.accessions as u64)
                        .bit_concat(y)
                        .to_bit_vec()
                } else if (x.len() > 0) && (y.len() == 0) {
                    x.bit_concat(&BitVec::new_fill(false, other_bigsi.accessions as u64))
                        .to_bit_vec()
                } else {
                    x.bit_concat(y).to_bit_vec()
                }
            })
            .collect();
        self.accessions = self.accessions + other_bigsi.accessions;
    }
    pub fn save(&self, file_name: &str) {
        let serialized: Vec<u8> = serialize(&self).unwrap();
        let mut writer = File::create(file_name).unwrap();
        writer
            .write_all(&serialized)
            .expect("problems preparing serialized data for writing");
    }
    pub fn read(&mut self, path: &str) {
        let mut reader = BufReader::new(File::open(path).expect("Can't open index!"));
        let bigsi: Bigsi = deserialize_from(&mut reader).expect("can't deserialize");
        self.bigsi = bigsi.bigsi;
        self.num_hashes = bigsi.num_hashes; // # of hashes needed
        self.accessions = bigsi.accessions;
    }
}

#[cfg(test)]
mod tests {
    use super::Bigsi;

    #[test]
    fn set_filter() {
        let new_bigsi = Bigsi::new(250000, 3, 10);
        println!("{}", new_bigsi.bigsi.len());
    }

    #[test]
    fn add_filter() {
        let mut new_filter = Bigsi::new(250000, 3, 10);
        new_filter.insert(0, "ATGC");
    }
    #[test]
    fn use_filter() {
        let mut new_filter = Bigsi::new(250000, 10, 3);
        new_filter.insert(0, "ATGT");
        new_filter.insert(3, "ATGT");
        new_filter.insert(7, "ATGT");
        new_filter.slim();
        assert_eq!(new_filter.get("ATGT").len(), 3 as usize);
        assert_eq!(new_filter.get("ATGC").len(), 0 as usize);
    }
    #[test]
    // we have to test the 0 length situations!
    fn merge_filters() {
        let mut new_filter = Bigsi::new(2500, 10, 3);
        new_filter.insert(0, "ATGT");
        new_filter.insert(3, "ATGT");
        new_filter.insert(7, "ATGT");
        //new_filter.slim();
        let mut second_filter = Bigsi::new(2500, 10, 3);
        second_filter.insert(0, "ATGT");
        second_filter.insert(3, "ATGT");
        second_filter.insert(7, "ATGT");
        //second_filter.slim();
        new_filter.merge(&second_filter);
        eprintln!("{} {}", new_filter.bigsi.len(), new_filter.bigsi[0].len());
        assert_eq!(new_filter.bigsi[0].len(), 20);
        //assert_eq!(new_filter.get("ATGT").len(), 3 as usize);
        //assert_eq!(new_filter.get("ATGC").len(), 0 as usize);
    }
    #[test]
    fn save_read_filter() {
        let mut new_filter = Bigsi::new(250000, 10, 3);
        new_filter.insert(0, "ATGT");
        new_filter.insert(3, "ATGT");
        new_filter.insert(7, "ATGT");
        new_filter.save("saved.bxi");
        let mut read_filter = Bigsi::default();
        read_filter.read("saved.bxi");
        assert_eq!(read_filter.get("ATGT").len(), 3 as usize);
        assert_eq!(read_filter.get("ATGC").len(), 0 as usize);
    }
}