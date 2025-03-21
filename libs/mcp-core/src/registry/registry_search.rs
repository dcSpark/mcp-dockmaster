use std::borrow::Cow;
use std::collections::HashMap;

// Import the probly-search library parts with public modules
use probly_search::Index;
use probly_search::score::{bm25, zero_to_one};

// Import the registry cache from the correct path
use crate::registry::registry_cache::RegistryCache;

// Import RegistryTool instead of Tool
use crate::models::types::{RegistryToolsResponse, RegistryTool};

/// A search service that builds an index from the cached registry and provides search functionality.
#[derive(Debug)]
pub struct RegistrySearch {
    // The search index, using a numeric key for each tool.
    index: Index<u32>,
    // A vector of tools. The position in this vector corresponds to the key used in the index.
    tools: Vec<RegistryTool>,
}

#[derive(thiserror::Error, Debug)]
pub enum SearchError {
    #[error("Registry cache error: {0}")]
    CacheError(String),
    #[error("Invalid query: {0}")]
    QueryError(String),
    #[error("Index error: {0}")]
    IndexError(String),
}

impl RegistrySearch {
    /// Create a new RegistrySearch instance by loading the registry from the cache and indexing each tool.
    pub async fn new() -> Result<Self, SearchError> {
        // Fetch registry data (this uses the async method from your cache implementation).
        let registry: RegistryToolsResponse = RegistryCache::instance().get_registry_tools().await
            .map_err(|e| SearchError::CacheError(e.message))?;
        let tools = registry.tools;

        // Create a new index with 4 fields: title, description, publisher, categories.
        let mut index = Index::<u32>::new(4);

        // For each tool, assign a numeric key and add it to the index.
        for (i, tool) in tools.iter().enumerate() {
            let key = i as u32;
            
            // Define extraction functions for each field in advance to avoid lifetime issues
            type FA<'a> = for<'b> fn(&'b RegistryTool) -> Vec<&'b str>;
            
            let title_extractor: FA = |t| vec![&t.name];
            let desc_extractor: FA = |t| vec![&t.description];
            let publisher_extractor: FA = |t| vec![&t.publisher.name];
            let categories_extractor: FA = |t| t.categories.iter().map(|s| s.as_str()).collect();
            
            let extractors: [FA; 4] = [
                title_extractor,
                desc_extractor, 
                publisher_extractor,
                categories_extractor
            ];
            
            // Add the document to the index
            index.add_document(&extractors, Self::tokenizer, key, tool);
        }

        Ok(RegistrySearch { index, tools })
    }

    /// A simple tokenizer: converts the input to lowercase and splits on whitespace.
    fn tokenizer(s: &str) -> Vec<Cow<'static, str>> {
        // Convert to owned strings to avoid lifetime issues
        s.to_lowercase()
            .split_whitespace()
            .map(|word| Cow::Owned(word.to_string()))
            .collect()
    }

    /// Search the registry using the given query string.
    ///
    /// This method runs the query using both BM25 and Zero-to-One scoring, combines the results,
    /// and returns a vector of tools along with their combined score (sorted in descending order).
    pub fn search(&mut self, query: &str) -> Result<Vec<(RegistryTool, f64)>, SearchError> {
        // Add input validation
        if query.trim().is_empty() {
            return Err(SearchError::QueryError("Search query cannot be empty".to_string()));
        }

        // Make weights configurable or const
        const WEIGHTS: [f64; 4] = [3.0, 2.0, 0.5, 0.5];
        const BM25_WEIGHT: f64 = 0.7;
        const ZTO_WEIGHT: f64 = 0.3;

        // Cache scorer instances
        let mut bm25_scorer = bm25::new();
        let mut zto1_scorer = zero_to_one::new();

        // Run queries sequentially instead of using rayon
        let bm25_results = self.index.query(query, &mut bm25_scorer, Self::tokenizer, &WEIGHTS);
        let zero_to_one_results = self.index.query(query, &mut zto1_scorer, Self::tokenizer, &WEIGHTS);

        // More efficient score combination
        let mut combined_scores: HashMap<u32, f64> = HashMap::with_capacity(
            bm25_results.len().max(zero_to_one_results.len())
        );

        for res in bm25_results {
            combined_scores.insert(res.key, BM25_WEIGHT * res.score);
        }

        for res in zero_to_one_results {
            combined_scores
                .entry(res.key)
                .and_modify(|s| *s += ZTO_WEIGHT * res.score)
                .or_insert(ZTO_WEIGHT * res.score);
        }

        // Add result limit option
        let mut results: Vec<(RegistryTool, f64)> = combined_scores
            .into_iter()
            .filter_map(|(key, score)| {
                self.tools.get(key as usize)
                    .map(|tool| (tool.clone(), score))
            })
            .collect();
        
        results.sort_unstable_by(|a, b| 
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        );
        
        Ok(results)
    }

    /// Rebuild the search index from the updated registry cache.
    ///
    /// This method fetches the latest registry data from the cache and rebuilds the in-memory index.
    pub async fn rebuild_index(&mut self) -> Result<(), SearchError> {
        // Add capacity hint
        let registry: RegistryToolsResponse = RegistryCache::instance().get_registry_tools().await
            .map_err(|e| SearchError::CacheError(e.message))?;
        let tools = registry.tools;
        
        // Create a new index with the same number of fields
        let mut new_index = Index::<u32>::new(4);

        // For each tool, assign a numeric key and add it to the index.
        for (i, tool) in tools.iter().enumerate() {
            let key = i as u32;
            
            // Define extraction functions for each field in advance to avoid lifetime issues
            type FA<'a> = for<'b> fn(&'b RegistryTool) -> Vec<&'b str>;
            
            let title_extractor: FA = |t| vec![&t.name];
            let desc_extractor: FA = |t| vec![&t.description];
            let publisher_extractor: FA = |t| vec![&t.publisher.name];
            let categories_extractor: FA = |t| t.categories.iter().map(|s| s.as_str()).collect();
            
            let extractors: [FA; 4] = [
                title_extractor,
                desc_extractor, 
                publisher_extractor,
                categories_extractor
            ];
            
            // Add the document to the index
            new_index.add_document(&extractors, Self::tokenizer, key, tool);
        }

        // Atomic update
        self.index = new_index;
        self.tools = tools;
        
        Ok(())
    }
}
