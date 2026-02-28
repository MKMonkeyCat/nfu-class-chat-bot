use jieba_rs::{Jieba, KeywordExtract, TfIdf};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

fn calculate_hash(word: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    word.hash(&mut hasher);
    hasher.finish()
}

pub struct FingerprintEngine {
    jieba: Jieba,
    tfidf: TfIdf,
}

impl FingerprintEngine {
    pub fn new() -> Self {
        Self {
            jieba: Jieba::new(),
            tfidf: TfIdf::default(),
        }
    }

    pub fn generate(&self, text: &str) -> u64 {
        let mut weights = [0.0f64; 64];
        let char_count = text.chars().count();

        if char_count < 100 {
            let words = self.jieba.cut(text, false);
            for word in words {
                let h = calculate_hash(word);
                let w = if word.len() <= 3 && word.chars().all(|c| c.is_ascii_punctuation()) {
                    0.05
                } else {
                    1.0
                };
                Self::apply_hash_to_weights(&mut weights, h, w);
            }
        } else {
            let keywords = self.tfidf.extract_keywords(&self.jieba, text, 30, vec![]);
            for kw in keywords {
                let h = calculate_hash(&kw.keyword);
                Self::apply_hash_to_weights(&mut weights, h, kw.weight);
            }
        }

        let mut fingerprint = 0u64;
        for i in 0..64 {
            if weights[i] > 0.0 {
                fingerprint |= 1 << i;
            }
        }
        fingerprint
    }

    fn apply_hash_to_weights(weights: &mut [f64; 64], h: u64, w: f64) {
        for i in 0..64 {
            if (h >> i) & 1 == 1 {
                weights[i] += w;
            } else {
                weights[i] -= w;
            }
        }
    }
}

pub struct FingerprintIndex {
    blocks: [HashMap<u16, Vec<(u64, usize)>>; 4],
}

impl FingerprintIndex {
    pub fn new() -> Self {
        Self {
            blocks: [
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
            ],
        }
    }

    fn split_fingerprint(fp: u64) -> [u16; 4] {
        [
            (fp & 0xFFFF) as u16,
            ((fp >> 16) & 0xFFFF) as u16,
            ((fp >> 32) & 0xFFFF) as u16,
            ((fp >> 48) & 0xFFFF) as u16,
        ]
    }

    pub fn hamming_distance(lhs: u64, rhs: u64) -> u32 {
        (lhs ^ rhs).count_ones()
    }

    pub fn insert(&mut self, fp: u64, doc_id: usize) {
        let chunks = Self::split_fingerprint(fp);
        for (i, &chunk) in chunks.iter().enumerate() {
            self.blocks[i].entry(chunk).or_default().push((fp, doc_id));
        }
    }

    pub fn search(&self, fp: u64, threshold: u32) -> Vec<(usize, u32)> {
        let mut found = HashMap::new();
        let chunks = Self::split_fingerprint(fp);

        for (i, &chunk) in chunks.iter().enumerate() {
            if let Some(candidates) = self.blocks[i].get(&chunk) {
                for &(cand_fp, doc_id) in candidates {
                    let dist = Self::hamming_distance(fp, cand_fp);
                    if dist <= threshold {
                        found.insert(doc_id, dist);
                    }
                }
            }
        }

        if found.is_empty() && threshold > 12 {
            for block in &self.blocks {
                for candidates in block.values() {
                    for &(cand_fp, doc_id) in candidates {
                        let dist = Self::hamming_distance(fp, cand_fp);
                        if dist <= threshold {
                            found.insert(doc_id, dist);
                        }
                    }
                }
            }
        }

        let mut res: Vec<_> = found.into_iter().collect();
        res.sort_by_key(|&(_, dist)| dist);
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simhash_simplified_chinese_logic() {
        let engine = FingerprintEngine::new();
        let mut index = FingerprintIndex::new();

        let doc_0 = "今天天气非常好，我想去公园散步吃苹果。";
        let doc_1 = "天气挺不错的今天，我打算去公园走走吃个苹果。";
        let doc_2 = "今天天气好，去公园散步吃苹果。";
        let doc_3 = "人工智能是未来科技发展的核心驱动力，深度学习已广泛应用。";
        let doc_4 = "红酒炖牛肉做法：首先准备新鲜牛肉，切块後放入红酒腌制。";

        let fp_0 = engine.generate(doc_0);
        let fp_1 = engine.generate(doc_1);
        let fp_2 = engine.generate(doc_2);
        let fp_3 = engine.generate(doc_3);
        let fp_4 = engine.generate(doc_4);

        let threshold = 16;

        let dist_1 = FingerprintIndex::hamming_distance(fp_0, fp_1);
        assert!(
            dist_1 < threshold,
            "Distance too large after word reordering: {}",
            dist_1
        );

        let dist_2 = FingerprintIndex::hamming_distance(fp_0, fp_2);
        assert!(
            dist_2 < threshold,
            "Distance too large after text reduction: {}",
            dist_2
        );

        let dist_3 = FingerprintIndex::hamming_distance(fp_0, fp_3);
        assert!(
            dist_3 > threshold,
            "Distance too small for completely different topics: {}",
            dist_3
        );

        index.insert(fp_0, 0);
        index.insert(fp_3, 3);
        index.insert(fp_4, 4);

        let matches = index.search(fp_1, threshold);
        let match_ids: Vec<usize> = matches.iter().map(|m| m.0).collect();

        assert!(
            match_ids.contains(&0),
            "Should match the baseline document (ID 0)"
        );
        assert!(
            !match_ids.contains(&3),
            "Should not match unrelated AI document (ID 3)"
        );
        assert!(
            !match_ids.contains(&4),
            "Should not match unrelated recipe document (ID 4)"
        );
    }
}
