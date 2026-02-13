//! Semantic search using TF-IDF vectors and cosine similarity.
//! Provides meaning-aware search beyond keyword matching.
use std::collections::HashMap;
use std::sync::RwLock;

/// A document in the TF-IDF index.
#[derive(Clone)]
struct Document {
    post_id: String,
    blog_id: String,
    tfidf: HashMap<String, f64>,
    magnitude: f64,
}

struct IndexData {
    documents: Vec<Document>,
    idf: HashMap<String, f64>,
    doc_count: usize,
}

pub struct SemanticIndex {
    inner: RwLock<IndexData>,
}

/// A search hit with similarity score.
pub struct SemanticHit {
    pub post_id: String,
    pub blog_id: String,
    pub similarity: f64,
}

// ─── Stop Words ───

const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
    "of", "with", "by", "is", "it", "as", "be", "this", "that", "from",
    "was", "are", "were", "been", "has", "have", "had", "not", "no", "do",
    "does", "did", "will", "would", "can", "could", "should", "may", "might",
    "i", "we", "you", "he", "she", "they", "my", "your", "how", "what",
    "why", "when", "where", "which", "who", "its", "their", "our", "his",
    "her", "them", "us", "me", "than", "then", "so", "if", "about", "up",
    "out", "just", "also", "more", "some", "any", "all", "each", "every",
    "into", "over", "after", "before", "between", "through", "during",
    "very", "most", "other", "such", "only", "same", "own", "both",
    "being", "here", "there", "these", "those", "while", "because",
];

// ─── Simple Stemmer ───

/// Basic suffix-stripping stemmer for English. Handles common suffixes
/// to normalize related words (e.g., "running" → "run", "deployment" → "deploy").
fn stem(word: &str) -> String {
    let w = word.to_lowercase();
    if w.len() < 4 {
        return w;
    }

    // Order matters: try longest suffixes first
    let suffixes: &[(&str, &str)] = &[
        ("ational", "ate"),
        ("tional", "tion"),
        ("encies", "ence"),
        ("ancies", "ance"),
        ("fulness", "ful"),
        ("ousness", "ous"),
        ("iveness", "ive"),
        ("ization", "ize"),
        ("ements", "e"),
        ("nesses", "ness"),
        ("ments", "ment"),
        ("ating", "ate"),
        ("ition", "it"),
        ("ising", "ise"),
        ("izing", "ize"),
        ("ation", "ate"),
        ("ities", "ity"),
        ("ously", "ous"),
        ("ively", "ive"),
        ("fully", "ful"),
        ("ings", ""),
        ("ment", ""),
        ("ness", ""),
        ("ting", "t"),
        ("able", ""),
        ("ible", ""),
        ("ally", "al"),
        ("ence", ""),
        ("ance", ""),
        ("ious", ""),
        ("eous", ""),
        ("ing", ""),
        ("ies", "y"),
        ("ion", ""),
        ("ful", ""),
        ("ous", ""),
        ("ive", ""),
        ("ize", ""),
        ("ise", ""),
        ("ate", ""),
        ("ity", ""),
        ("ers", ""),
        ("est", ""),
        ("ess", ""),
        ("ism", ""),
        ("ist", ""),
        ("ant", ""),
        ("ent", ""),
        ("ed", ""),
        ("er", ""),
        ("ly", ""),
        ("es", ""),
        ("al", ""),
        ("s", ""),
    ];

    for (suffix, replacement) in suffixes {
        if w.ends_with(suffix) {
            let base = &w[..w.len() - suffix.len()];
            if base.len() >= 2 {
                return format!("{}{}", base, replacement);
            }
        }
    }

    w
}

// ─── Tokenizer ───

/// Tokenize text into stemmed, lower-case terms with stop words removed.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2 && !STOP_WORDS.contains(w))
        .map(stem)
        .collect()
}

// ─── TF-IDF Computation ───

/// Compute term frequency for a token list (normalized by doc length).
fn term_frequency(tokens: &[String]) -> HashMap<String, f64> {
    let mut counts: HashMap<String, f64> = HashMap::new();
    for token in tokens {
        *counts.entry(token.clone()).or_insert(0.0) += 1.0;
    }
    let len = tokens.len() as f64;
    if len > 0.0 {
        for v in counts.values_mut() {
            *v /= len;
        }
    }
    counts
}

/// Compute IDF values from a set of document term-frequency maps.
fn compute_idf(docs: &[HashMap<String, f64>]) -> HashMap<String, f64> {
    let n = docs.len() as f64;
    let mut df: HashMap<String, f64> = HashMap::new();
    for doc in docs {
        for term in doc.keys() {
            *df.entry(term.clone()).or_insert(0.0) += 1.0;
        }
    }
    let mut idf = HashMap::new();
    for (term, count) in df {
        // Smooth IDF: log((N + 1) / (df + 1)) + 1
        idf.insert(term, ((n + 1.0) / (count + 1.0)).ln() + 1.0);
    }
    idf
}

/// Compute TF-IDF vector and its magnitude.
fn compute_tfidf(tf: &HashMap<String, f64>, idf: &HashMap<String, f64>) -> (HashMap<String, f64>, f64) {
    let mut tfidf = HashMap::new();
    let mut mag_sq = 0.0;
    for (term, &freq) in tf {
        let idf_val = idf.get(term).copied().unwrap_or(1.0);
        let weight = freq * idf_val;
        mag_sq += weight * weight;
        tfidf.insert(term.clone(), weight);
    }
    (tfidf, mag_sq.sqrt())
}

/// Cosine similarity between a query TF-IDF vector and a document.
fn cosine_similarity(query: &HashMap<String, f64>, q_mag: f64, doc: &Document) -> f64 {
    if q_mag == 0.0 || doc.magnitude == 0.0 {
        return 0.0;
    }
    let dot: f64 = query.iter()
        .filter_map(|(term, &qw)| doc.tfidf.get(term).map(|&dw| qw * dw))
        .sum();
    dot / (q_mag * doc.magnitude)
}

// ─── Public API ───

/// Data needed to add a post to the index.
pub struct PostData {
    pub post_id: String,
    pub blog_id: String,
    pub title: String,
    pub content: String,
    pub tags: String,
    pub summary: String,
}

impl Default for SemanticIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticIndex {
    pub fn new() -> Self {
        SemanticIndex {
            inner: RwLock::new(IndexData {
                documents: Vec::new(),
                idf: HashMap::new(),
                doc_count: 0,
            }),
        }
    }

    /// Rebuild the entire index from a list of posts.
    pub fn rebuild(&self, posts: Vec<PostData>) {
        // Build combined text for each post and compute TF
        let mut tf_maps: Vec<(PostData, HashMap<String, f64>)> = Vec::new();
        for post in posts {
            let combined = format!(
                "{title} {title} {title} {summary} {summary} {tags} {content}",
                title = post.title,
                summary = post.summary,
                tags = post.tags,
                content = post.content,
            );
            let tokens = tokenize(&combined);
            let tf = term_frequency(&tokens);
            tf_maps.push((post, tf));
        }

        // Compute IDF across all documents
        let all_tf: Vec<HashMap<String, f64>> = tf_maps.iter().map(|(_, tf)| tf.clone()).collect();
        let idf = compute_idf(&all_tf);

        // Build TF-IDF vectors
        let documents: Vec<Document> = tf_maps.into_iter()
            .map(|(post, tf)| {
                let (tfidf, magnitude) = compute_tfidf(&tf, &idf);
                Document {
                    post_id: post.post_id,
                    blog_id: post.blog_id,
                    tfidf,
                    magnitude,
                }
            })
            .collect();

        let doc_count = documents.len();
        let mut data = self.inner.write().unwrap();
        data.documents = documents;
        data.idf = idf;
        data.doc_count = doc_count;
    }

    /// Search across all indexed posts. Returns hits sorted by similarity descending.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SemanticHit> {
        let tokens = tokenize(query);
        if tokens.is_empty() {
            return Vec::new();
        }

        let data = self.inner.read().unwrap();
        let tf = term_frequency(&tokens);
        let (q_tfidf, q_mag) = compute_tfidf(&tf, &data.idf);

        let mut hits: Vec<SemanticHit> = data.documents.iter()
            .map(|doc| SemanticHit {
                post_id: doc.post_id.clone(),
                blog_id: doc.blog_id.clone(),
                similarity: cosine_similarity(&q_tfidf, q_mag, doc),
            })
            .filter(|h| h.similarity > 0.01)
            .collect();

        hits.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(limit);
        hits
    }

    /// Search within a specific blog. Returns hits sorted by similarity descending.
    pub fn search_blog(&self, blog_id: &str, query: &str, limit: usize) -> Vec<SemanticHit> {
        let tokens = tokenize(query);
        if tokens.is_empty() {
            return Vec::new();
        }

        let data = self.inner.read().unwrap();
        let tf = term_frequency(&tokens);
        let (q_tfidf, q_mag) = compute_tfidf(&tf, &data.idf);

        let mut hits: Vec<SemanticHit> = data.documents.iter()
            .filter(|doc| doc.blog_id == blog_id)
            .map(|doc| SemanticHit {
                post_id: doc.post_id.clone(),
                blog_id: doc.blog_id.clone(),
                similarity: cosine_similarity(&q_tfidf, q_mag, doc),
            })
            .filter(|h| h.similarity > 0.01)
            .collect();

        hits.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(limit);
        hits
    }

    /// Find posts similar to a given post. Used by related posts endpoint.
    #[allow(dead_code)]
    pub fn find_similar(&self, post_id: &str, blog_id: &str, limit: usize) -> Vec<SemanticHit> {
        let data = self.inner.read().unwrap();

        let source = match data.documents.iter().find(|d| d.post_id == post_id) {
            Some(doc) => doc,
            None => return Vec::new(),
        };

        let mut hits: Vec<SemanticHit> = data.documents.iter()
            .filter(|doc| doc.blog_id == blog_id && doc.post_id != post_id)
            .map(|doc| {
                // Direct cosine similarity between document vectors
                let dot: f64 = source.tfidf.iter()
                    .filter_map(|(term, &sw)| doc.tfidf.get(term).map(|&dw| sw * dw))
                    .sum();
                let sim = if source.magnitude > 0.0 && doc.magnitude > 0.0 {
                    dot / (source.magnitude * doc.magnitude)
                } else {
                    0.0
                };
                SemanticHit {
                    post_id: doc.post_id.clone(),
                    blog_id: doc.blog_id.clone(),
                    similarity: sim,
                }
            })
            .filter(|h| h.similarity > 0.01)
            .collect();

        hits.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(limit);
        hits
    }

    /// Add or update a single post in the index. Triggers full IDF recomputation.
    pub fn upsert(&self, post: PostData) {
        let mut data = self.inner.write().unwrap();

        // Remove existing entry if any
        data.documents.retain(|d| d.post_id != post.post_id);

        // Build text and TF for new post
        let combined = format!(
            "{title} {title} {title} {summary} {summary} {tags} {content}",
            title = post.title,
            summary = post.summary,
            tags = post.tags,
            content = post.content,
        );
        let tokens = tokenize(&combined);
        let tf = term_frequency(&tokens);

        // Recompute IDF with the new document included
        let mut all_tf: Vec<HashMap<String, f64>> = data.documents.iter()
            .map(|d| d.tfidf.clone()) // This is an approximation; we store TF-IDF not TF
            .collect();
        all_tf.push(tf.clone());
        let idf = compute_idf(&all_tf);

        // Rebuild all TF-IDF vectors with new IDF
        // (For large corpora this would be expensive, but blogs are small)
        let (tfidf, magnitude) = compute_tfidf(&tf, &idf);
        data.documents.push(Document {
            post_id: post.post_id,
            blog_id: post.blog_id,
            tfidf,
            magnitude,
        });

        // Update global IDF and recompute magnitudes for existing docs
        let old_idf = data.idf.clone();
        for doc in data.documents.iter_mut() {
            let mut mag_sq = 0.0;
            for (term, weight) in doc.tfidf.iter_mut() {
                // Approximate: scale existing weights by new IDF ratio
                if let Some(&new_idf_val) = idf.get(term) {
                    let old_idf_val = old_idf.get(term).copied().unwrap_or(1.0);
                    if old_idf_val > 0.0 {
                        *weight *= new_idf_val / old_idf_val;
                    }
                }
                mag_sq += *weight * *weight;
            }
            doc.magnitude = mag_sq.sqrt();
        }

        data.idf = idf;
        data.doc_count = data.documents.len();
    }

    /// Remove a post from the index.
    pub fn remove(&self, post_id: &str) {
        let mut data = self.inner.write().unwrap();
        data.documents.retain(|d| d.post_id != post_id);
        data.doc_count = data.documents.len();
        // IDF not recomputed on removal — acceptable for small corpus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_post(id: &str, blog_id: &str, title: &str, content: &str, tags: &str) -> PostData {
        PostData {
            post_id: id.to_string(),
            blog_id: blog_id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            tags: tags.to_string(),
            summary: String::new(),
        }
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Running quickly through the deployment pipeline");
        assert!(!tokens.is_empty());
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"through".to_string()));
    }

    #[test]
    fn test_stem_basic() {
        assert_eq!(stem("running"), "runn"); // -ing removed
        assert_eq!(stem("deployment"), "deploy"); // -ment removed
        assert_eq!(stem("quickly"), "quick"); // -ly removed
        assert_eq!(stem("tests"), "test"); // -s removed
    }

    #[test]
    fn test_empty_index() {
        let index = SemanticIndex::new();
        let results = index.search("hello world", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_basic_search() {
        let index = SemanticIndex::new();
        index.rebuild(vec![
            make_post("1", "b1", "Rust Programming Guide", "Learn Rust programming language with examples and exercises", "rust,programming"),
            make_post("2", "b1", "Python Data Science", "Python for data science machine learning and analytics", "python,data"),
            make_post("3", "b1", "Web Development with JavaScript", "Building web applications using JavaScript and React framework", "javascript,web"),
        ]);

        let results = index.search("rust programming", 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].post_id, "1");
    }

    #[test]
    fn test_semantic_similarity() {
        let index = SemanticIndex::new();
        index.rebuild(vec![
            make_post("1", "b1", "Machine Learning Basics", "Introduction to neural networks deep learning and AI models", "ml,ai"),
            make_post("2", "b1", "Artificial Intelligence Overview", "AI systems neural networks and deep learning algorithms", "ai,deep-learning"),
            make_post("3", "b1", "Cooking Italian Food", "Recipes for pasta pizza and traditional Italian cuisine", "cooking,food"),
        ]);

        // "neural network AI" should match posts 1 and 2, not 3
        let results = index.search("neural network AI", 10);
        assert!(results.len() >= 2);
        let ids: Vec<&str> = results.iter().map(|r| r.post_id.as_str()).collect();
        assert!(ids.contains(&"1"));
        assert!(ids.contains(&"2"));
        // Post 3 (cooking) should not appear or be ranked very low
        if let Some(cooking) = results.iter().find(|r| r.post_id == "3") {
            assert!(cooking.similarity < results[0].similarity * 0.5);
        }
    }

    #[test]
    fn test_find_similar() {
        let index = SemanticIndex::new();
        index.rebuild(vec![
            make_post("1", "b1", "Rust Web Frameworks", "Comparing Rocket Actix and Axum for web development in Rust", "rust,web"),
            make_post("2", "b1", "Building APIs with Rocket", "How to build REST APIs using the Rocket framework in Rust", "rust,api"),
            make_post("3", "b1", "Gardening Tips", "How to grow tomatoes and herbs in your backyard garden", "garden,tips"),
        ]);

        let similar = index.find_similar("1", "b1", 10);
        assert!(!similar.is_empty());
        // Post 2 (also about Rust web) should be more similar than post 3 (gardening)
        assert_eq!(similar[0].post_id, "2");
    }

    #[test]
    fn test_upsert_and_remove() {
        let index = SemanticIndex::new();
        index.rebuild(vec![
            make_post("1", "b1", "Hello World", "A first post about programming", "intro"),
        ]);

        assert_eq!(index.search("programming", 10).len(), 1);

        // Add another post
        index.upsert(make_post("2", "b1", "Advanced Programming", "Deep dive into programming concepts and patterns", "programming"));
        let results = index.search("programming", 10);
        assert!(results.len() >= 2);

        // Remove original
        index.remove("1");
        let results = index.search("programming", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].post_id, "2");
    }

    #[test]
    fn test_search_blog_filter() {
        let index = SemanticIndex::new();
        index.rebuild(vec![
            make_post("1", "b1", "Rust Guide", "Rust programming language", "rust"),
            make_post("2", "b2", "Rust Cookbook", "Rust recipes and patterns", "rust"),
        ]);

        let all = index.search("rust", 10);
        assert_eq!(all.len(), 2);

        let b1_only = index.search_blog("b1", "rust", 10);
        assert_eq!(b1_only.len(), 1);
        assert_eq!(b1_only[0].post_id, "1");
    }
}
