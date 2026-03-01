use jieba_rs::{Jieba, KeywordExtract, TfIdf};
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

    pub fn split_fingerprint(fp: u64) -> [u16; 4] {
        [
            (fp & 0xFFFF) as u16,
            ((fp >> 16) & 0xFFFF) as u16,
            ((fp >> 32) & 0xFFFF) as u16,
            ((fp >> 48) & 0xFFFF) as u16,
        ]
    }

    pub fn hamming_distance(fp1: u64, fp2: u64) -> u32 {
        (fp1 ^ fp2).count_ones()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simhash_simplified_chinese_logic() {
        let engine = FingerprintEngine::new();

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

        let dist_0_1 = FingerprintEngine::hamming_distance(fp_0, fp_1);
        let dist_0_2 = FingerprintEngine::hamming_distance(fp_0, fp_2);
        let dist_0_3 = FingerprintEngine::hamming_distance(fp_0, fp_3);
        let dist_0_4 = FingerprintEngine::hamming_distance(fp_0, fp_4);

        // println!("Distance between doc_0 and doc_1: {}", dist_0_1); // 15
        // println!("Distance between doc_0 and doc_2: {}", dist_0_2); // 8
        // println!("Distance between doc_0 and doc_3: {}", dist_0_3); // 34
        // println!("Distance between doc_0 and doc_4: {}", dist_0_4); // 31

        let threshold = 16;
        assert!(dist_0_1 <= threshold);
        assert!(dist_0_2 <= threshold);
        assert!(dist_0_3 > threshold);
        assert!(dist_0_4 > threshold);

        assert!(dist_0_1 == 15);
        assert!(dist_0_2 == 8);
        assert!(dist_0_3 == 34);
        assert!(dist_0_4 == 31);
    }
}
