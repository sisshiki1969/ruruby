//use crate::vm::vm_inst::Inst;
use crate::*;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;

#[derive(Debug)]
pub struct FiberInfo {
    vm: VMRef,
    context: ContextRef,
    rec: Receiver<VMResult>,
    tx: SyncSender<usize>,
}

pub type FiberRef = Ref<FiberInfo>;

impl FiberInfo {
    pub fn new(
        vm: VMRef,
        context: ContextRef,
        rec: Receiver<VMResult>,
        tx: SyncSender<usize>,
    ) -> Self {
        FiberInfo {
            vm,
            context,
            rec,
            tx,
        }
    }
}

pub fn init_fiber(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Fiber");
    let class = ClassRef::from(id, globals.builtins.object);
    let val = Value::class(globals, class);
    globals.add_builtin_instance_method(class, "resume", resume);
    globals.add_builtin_class_method(val, "new", new);
    globals.add_builtin_class_method(val, "yield", yield_);
    val
}

// Class methods

fn new(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let method = vm.expect_block(args.block)?;
    let context = vm.create_block_context(method)?;
    let (tx0, rx0) = std::sync::mpsc::sync_channel(0);
    let (tx1, rx1) = std::sync::mpsc::sync_channel(0);
    let mut new_vm = vm.dup();
    new_vm.clear_();
    new_vm.fiberstate_created();
    new_vm.sender = Some((tx0, rx1));
    let val = Value::fiber(&vm.globals, VMRef::new(new_vm), context, rx0, tx1);
    Ok(val)
}

fn yield_(vm: &mut VM, args: &Args) -> VMResult {
    let val = match args.len() {
        0 => Value::nil(),
        1 => args[0],
        _ => {
            let mut ary = vec![];
            for i in 0..args.len() {
                ary.push(args[i]);
            }
            Value::array_from(&vm.globals, ary)
        }
    };
    match &vm.sender {
        Some((tx, rx)) => {
            //eprintln!("sending..");
            tx.send(Ok(val)).unwrap();
            //eprintln!("sent.");
            rx.recv().unwrap();
        }
        None => {
            return Err(vm.error_fiber("Can not yield from main fiber."));
        }
    };
    //let mut context = vm.context();
    //let pc = vm.get_pc();
    //let inst = &context.iseq_ref.iseq[pc];
    //context.pc = pc + Inst::inst_size(*inst);
    //vm.stack_push(Value::nil());
    #[cfg(feature = "trace")]
    {
        println!("++YIELD++");
    }
    Ok(Value::nil())
}

// Instance methods

fn resume(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    match args.self_value.unpack() {
        RV::Object(obj) => match &obj.kind {
            ObjKind::Fiber(fiber) => {
                let mut context = fiber.context;
                context.is_fiber = true;
                let mut fiber_vm = fiber.vm;
                match fiber_vm.fiberstate() {
                    FiberState::Dead => {
                        return Err(vm.error_fiber("Dead fiber called."));
                    }
                    FiberState::Created => {
                        fiber_vm.fiberstate_running();
                        #[cfg(feature = "trace")]
                        {
                            println!("++SPAWN++");
                        }
                        let mut vm2 = fiber_vm;
                        thread::spawn(move || vm2.vm_run_context(context));
                        //eprintln!("receiving..");
                        let res = fiber.rec.recv().unwrap()?;
                        //eprintln!("received.");
                        return Ok(res);
                    }
                    FiberState::Running => {
                        #[cfg(feature = "trace")]
                        {
                            println!("++RESUME++");
                        }
                        fiber.tx.send(1).unwrap();
                        //eprintln!("receiving..");
                        let res = fiber.rec.recv().unwrap()?;
                        //eprintln!("received.");
                        return Ok(res);
                    }
                }
            }
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
}
