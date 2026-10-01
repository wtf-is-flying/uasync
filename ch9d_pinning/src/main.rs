use std::{marker::PhantomPinned, mem::swap, pin::Pin};

#[derive(Default)]
struct MaybeSelfRef {
    a: usize,
    b: Option<*mut usize>,
    _pin: PhantomPinned,
}

impl MaybeSelfRef {
    fn init(self: Pin<&mut Self>) {
        let Self { a, b, .. } = unsafe { self.get_unchecked_mut() };
        *b = Some(a);
    }

    fn b(self: Pin<&mut Self>) -> Option<&mut usize> {
        unsafe { self.get_unchecked_mut().b.map(|b| &mut *b) }
    }
}

fn heap_pinning() {
    let mut x = Box::pin(MaybeSelfRef::default());
    x.as_mut().init();
    println!("{}", x.as_ref().a);
    *x.as_mut().b().unwrap() = 2;
    println!("{}", x.as_ref().a);
}

fn stack_pinning_manual() {
    let mut x = MaybeSelfRef::default();
    let mut x = unsafe { Pin::new_unchecked(&mut x) };
    x.as_mut().init();
    println!("{}", x.as_ref().a);
    *x.as_mut().b().unwrap() = 2;
    println!("{}", x.as_ref().a);
}

fn stack_pinning_manual_problem() {
    let mut x = MaybeSelfRef::default();
    let mut y = MaybeSelfRef::default();
    {
        let mut x = unsafe { Pin::new_unchecked(&mut x) };
        x.as_mut().init();
        *x.as_mut().b().unwrap() = 2;
    }
    swap(&mut x, &mut y);
    println!(
        "
     x: {{
  +----->a: {:p},
  |      b: {:?},
  |  }}
  |
  |  y: {{
  |      a: {:p},
  +-----|b: {:?},
     }}",
        &x.a, x.b, &y.a, y.b,
    );
}

fn main() {
    heap_pinning();
    stack_pinning_manual();
    stack_pinning_manual_problem();
}
