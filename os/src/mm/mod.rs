//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{kernel_stack_position, MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, PageTableEntry};
use page_table::{PTEFlags, PageTable};

/// Represents different errors that can occur during memory mapping operations.
pub enum MapError {
    /// An error occurred when trying to find or create a page table entry.
    FindPteCreateError,
    /// Failed to allocate a physical frame for mapping.
    FrameAllocationFailed,
    /// Invalid permission bits were provided for the page table entry.
    InvalidPermissionBits(u8),
    /// Attempted to map a virtual page number that is already mapped.
    VpnAlreadyMapped(VirtPageNum),
    /// Failed to remove a mapped area.
    RemoveAreaFailed,
    /// Attempted to insert a mapped area that conflicts with an existing area.
    AreaConflict,
    /// The provided virtual address is not properly aligned.
    UnalignedVirtualAddress,
}

/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}
