/// Memory optimization and resource management utilities
use std::time::{Duration, Instant};
use std::sync::{Arc, atomic::{AtomicUsize, AtomicU64, Ordering}};
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

/// Memory manager for optimizing resource usage
pub struct MemoryManager {
    // Memory thresholds
    max_memory_mb: usize,
    warning_threshold: f64, // Percentage (0.0 - 1.0)
    critical_threshold: f64,
    
    // Monitoring
    last_check: Arc<RwLock<Instant>>,
    check_interval: Duration,
    
    // Metrics
    allocations: Arc<AtomicUsize>,
    deallocations: Arc<AtomicUsize>,
    peak_usage: Arc<AtomicUsize>,
    cleanup_count: Arc<AtomicU64>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(max_memory_mb: usize) -> Self {
        Self {
            max_memory_mb,
            warning_threshold: 0.8,
            critical_threshold: 0.95,
            last_check: Arc::new(RwLock::new(Instant::now())),
            check_interval: Duration::from_secs(30),
            allocations: Arc::new(AtomicUsize::new(0)),
            deallocations: Arc::new(AtomicUsize::new(0)),
            peak_usage: Arc::new(AtomicUsize::new(0)),
            cleanup_count: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Check memory usage and trigger cleanup if needed
    pub async fn check_memory_usage(&self) -> MemoryStatus {
        let mut last_check = self.last_check.write().await;
        let now = Instant::now();
        
        if now.duration_since(*last_check) < self.check_interval {
            return MemoryStatus::Normal;
        }
        
        *last_check = now;
        drop(last_check);
        
        // Get current memory usage (approximate)
        let current_usage = self.get_estimated_usage();
        let max_bytes = self.max_memory_mb * 1024 * 1024;
        let usage_ratio = current_usage as f64 / max_bytes as f64;
        
        // Update peak usage
        self.peak_usage.fetch_max(current_usage, Ordering::Relaxed);
        
        if usage_ratio >= self.critical_threshold {
            warn!("Critical memory usage: {:.1}% ({} MB)", 
                  usage_ratio * 100.0, current_usage / 1024 / 1024);
            MemoryStatus::Critical
        } else if usage_ratio >= self.warning_threshold {
            info!("High memory usage: {:.1}% ({} MB)", 
                  usage_ratio * 100.0, current_usage / 1024 / 1024);
            MemoryStatus::Warning
        } else {
            debug!("Normal memory usage: {:.1}% ({} MB)", 
                   usage_ratio * 100.0, current_usage / 1024 / 1024);
            MemoryStatus::Normal
        }
    }
    
    /// Get memory statistics
    pub fn get_memory_stats(&self) -> MemoryStats {
        let allocations = self.allocations.load(Ordering::Relaxed);
        let deallocations = self.deallocations.load(Ordering::Relaxed);
        let peak_usage = self.peak_usage.load(Ordering::Relaxed);
        let cleanup_count = self.cleanup_count.load(Ordering::Relaxed);
        let current_usage = self.get_estimated_usage();
        
        MemoryStats {
            current_usage_bytes: current_usage,
            peak_usage_bytes: peak_usage,
            max_allowed_bytes: self.max_memory_mb * 1024 * 1024,
            allocations,
            deallocations,
            net_allocations: allocations.saturating_sub(deallocations),
            cleanup_operations: cleanup_count,
            warning_threshold: self.warning_threshold,
            critical_threshold: self.critical_threshold,
        }
    }
    
    /// Record an allocation
    pub fn record_allocation(&self, size: usize) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        debug!("Recorded allocation of {} bytes", size);
    }
    
    /// Record a deallocation
    pub fn record_deallocation(&self, size: usize) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        debug!("Recorded deallocation of {} bytes", size);
    }
    
    /// Trigger cleanup operation
    pub async fn trigger_cleanup(&self) {
        self.cleanup_count.fetch_add(1, Ordering::Relaxed);
        info!("Memory cleanup triggered");
        
        // Force garbage collection hint (if supported)
        #[cfg(feature = "gc")]
        {
            std::gc::collect();
        }
    }
    
    /// Get estimated memory usage
    fn get_estimated_usage(&self) -> usize {
        // This is a simplified estimation
        // In a real implementation, you might use system calls or memory profiling
        let allocations = self.allocations.load(Ordering::Relaxed);
        let deallocations = self.deallocations.load(Ordering::Relaxed);
        
        // Rough estimate based on allocation tracking
        let net_allocs = allocations.saturating_sub(deallocations);
        net_allocs * 1024 // Assume average 1KB per allocation
    }
    
    /// Configure memory thresholds
    pub fn set_thresholds(&mut self, warning: f64, critical: f64) {
        self.warning_threshold = warning.clamp(0.0, 1.0);
        self.critical_threshold = critical.clamp(0.0, 1.0);
        
        if self.warning_threshold >= self.critical_threshold {
            self.warning_threshold = self.critical_threshold - 0.1;
        }
        
        info!("Updated memory thresholds: warning={:.1}%, critical={:.1}%", 
              self.warning_threshold * 100.0, self.critical_threshold * 100.0);
    }
}

/// Memory usage status
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryStatus {
    Normal,
    Warning,
    Critical,
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub current_usage_bytes: usize,
    pub peak_usage_bytes: usize,
    pub max_allowed_bytes: usize,
    pub allocations: usize,
    pub deallocations: usize,
    pub net_allocations: usize,
    pub cleanup_operations: u64,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
}

impl MemoryStats {
    /// Get current usage as a percentage
    pub fn usage_percentage(&self) -> f64 {
        if self.max_allowed_bytes == 0 {
            0.0
        } else {
            self.current_usage_bytes as f64 / self.max_allowed_bytes as f64
        }
    }
    
    /// Get peak usage as a percentage
    pub fn peak_usage_percentage(&self) -> f64 {
        if self.max_allowed_bytes == 0 {
            0.0
        } else {
            self.peak_usage_bytes as f64 / self.max_allowed_bytes as f64
        }
    }
    
    /// Check if memory usage is above warning threshold
    pub fn is_above_warning(&self) -> bool {
        self.usage_percentage() >= self.warning_threshold
    }
    
    /// Check if memory usage is critical
    pub fn is_critical(&self) -> bool {
        self.usage_percentage() >= self.critical_threshold
    }
}

/// Bounded collection that automatically manages memory
pub struct BoundedVec<T> {
    data: Vec<T>,
    max_size: usize,
    memory_manager: Option<Arc<MemoryManager>>,
}

impl<T> BoundedVec<T> {
    /// Create a new bounded vector
    pub fn new(max_size: usize) -> Self {
        Self {
            data: Vec::with_capacity(max_size),
            max_size,
            memory_manager: None,
        }
    }
    
    /// Create with memory manager integration
    pub fn with_memory_manager(max_size: usize, memory_manager: Arc<MemoryManager>) -> Self {
        Self {
            data: Vec::with_capacity(max_size),
            max_size,
            memory_manager: Some(memory_manager),
        }
    }
    
    /// Add an item, removing old ones if necessary
    pub fn push(&mut self, item: T) {
        if let Some(ref mm) = self.memory_manager {
            mm.record_allocation(std::mem::size_of::<T>());
        }
        
        if self.data.len() >= self.max_size {
            // Remove oldest item
            if let Some(_removed) = self.data.first() {
                if let Some(ref mm) = self.memory_manager {
                    mm.record_deallocation(std::mem::size_of::<T>());
                }
            }
            self.data.remove(0);
        }
        
        self.data.push(item);
    }
    
    /// Get current length
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }
    
    /// Clear all items
    pub fn clear(&mut self) {
        if let Some(ref mm) = self.memory_manager {
            let items_removed = self.data.len();
            mm.record_deallocation(items_removed * std::mem::size_of::<T>());
        }
        self.data.clear();
    }
    
    /// Shrink capacity to fit current size
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }
    
    /// Get iterator
    pub fn iter(&self) -> std::slice::Iter<T> {
        self.data.iter()
    }
    
    /// Get mutable iterator
    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        self.data.iter_mut()
    }
}

impl<T> std::ops::Index<usize> for BoundedVec<T> {
    type Output = T;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T> std::ops::IndexMut<usize> for BoundedVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_memory_manager_creation() {
        let manager = MemoryManager::new(100); // 100MB limit
        let stats = manager.get_memory_stats();
        
        assert_eq!(stats.max_allowed_bytes, 100 * 1024 * 1024);
        assert_eq!(stats.allocations, 0);
        assert_eq!(stats.deallocations, 0);
    }
    
    #[tokio::test]
    async fn test_memory_threshold_detection() {
        let mut manager = MemoryManager::new(1); // 1MB limit
        manager.set_thresholds(0.5, 0.8);
        
        // Test initial state
        let status = manager.check_memory_usage().await;
        assert_eq!(status, MemoryStatus::Normal);
        
        // Test that we can create the manager and it works
        let stats = manager.get_memory_stats();
        assert_eq!(stats.allocations, 0);
        assert_eq!(stats.deallocations, 0);
    }
    
    #[test]
    fn test_bounded_vec() {
        let mut vec = BoundedVec::new(3);
        
        vec.push(1);
        vec.push(2);
        vec.push(3);
        assert_eq!(vec.len(), 3);
        
        // Adding one more should remove the first
        vec.push(4);
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], 2);
        assert_eq!(vec[1], 3);
        assert_eq!(vec[2], 4);
    }
    
    #[test]
    fn test_memory_stats() {
        let manager = MemoryManager::new(100); // 100MB
        manager.record_allocation(1024);
        manager.record_allocation(2048);
        manager.record_deallocation(1024);
        
        let stats = manager.get_memory_stats();
        assert_eq!(stats.allocations, 2);
        assert_eq!(stats.deallocations, 1);
        assert_eq!(stats.net_allocations, 1);
    }
}