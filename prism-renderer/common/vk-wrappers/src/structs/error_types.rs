pub enum CommandPoolError {
    CreateError,
    BuffersAllocationError,
}

pub enum SemaphoreError {
    CreateError,
}

pub enum FenceError {
    CreateError,
    WaitError,
    ResetError,
}

pub enum AllocationError {
    LockError,
    AllocationFailed,
}

pub enum ImageError {
    CreateError,
    BindError,
}

pub enum ImageViewError{
    CreateError,
}

pub enum FrameBufferError{
    CreateError,
}

pub enum BufferError{
    CreateError,
    BindError,
}

pub enum DescriptorPoolError{
    CreateError,
    SetsAllocationError,
}

pub enum RendererError{
    CommandPool(CommandPoolError),
    Semaphore(SemaphoreError),
    Fence(FenceError),
    Allocation(AllocationError),
    Image(ImageError),
    ImageView(ImageViewError),
    FrameBuffer(FrameBufferError),
    Buffer(BufferError),
    DescriptorPool(DescriptorPoolError),
}
