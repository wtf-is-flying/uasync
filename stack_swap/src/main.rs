use core::arch::asm;

/// Stack size, in bytes.
const SSIZE: isize = 48;

#[derive(Debug, Default)]
#[repr(C)]
struct ThreadContext {
    /// Stack pointer register
    rsp: u64,
}

fn hello() -> ! {
    println!("I LOVE WAKING UP ON A NEW STACK!");
    loop {}
}

unsafe fn gt_switch(new: *const ThreadContext) {
    unsafe {
        asm!(
            // Assembly template.
            // Move the value stored at {0} + 0x00 to the rsp (stack) register.
            // [...] means "get what's at this memory location"
            "mov rsp, [{0} + 0x00]",
            // Pop a memory location off the stack, then jump to that location.
            "ret",
            // Let the compiler decide on a general purpose register
            // to store the value of new.
            in(reg) new,
        )
    }
}

fn main() {
    let mut ctx = ThreadContext::default();

    // Create a stack of SSIZE bytes
    let mut stack = vec![0_u8; SSIZE as usize];

    unsafe {
        // The unit of memory is a _byte_,
        // meaning there is 1 _byte_ of memory between address x and x + 1.
        // (It is not possible to get a pointer to bits of memory inside a byte.)

        // Stack grows downward, so SSIZE is the bottow of the stack.
        let stack_bottom = stack.as_mut_ptr().offset(SSIZE);

        // Clear the 4 lowest _bits_ of stack_bottom, making it a multiple of 2^4 = 16.
        // The address stack_bottom_aligned points to is thus a multiple of 16.
        // hence the name "16-bytes aligned".
        //
        // 15 = 000...01111 => !15 = 111..10000
        let stack_bottom_aligned = (stack_bottom as usize & !15) as *mut u8;

        // Write 0xFF to see where the bottom is.
        // std::ptr::write(stack_bottom_aligned, u8::MAX);

        // Write the function pointer `hello` to an offset of 16 bytes from the stack base.
        // `stack_bottom_aligned` is a pointer.
        // std::ptr::write(stack_bottom_aligned.offset(-16) as *mut u64, hello as u64);
        std::ptr::write(
            stack_bottom_aligned.offset(-16) as *mut u64,
            hello as *const () as u64,
        );

        // Set ctx's stack pointer to the address of `hello` we just wrote.
        ctx.rsp = stack_bottom_aligned.offset(-16) as u64;

        for i in 0..SSIZE {
            println!(
                "mem: {:x}, val: {:x}",
                stack_bottom_aligned.offset(-i) as usize,
                *stack_bottom_aligned.offset(-i)
            )
        }

        gt_switch(&ctx);
    }
}
