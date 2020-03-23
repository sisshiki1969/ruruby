use crate::vm::vm_inst::Inst;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct FiberInfo {
    vm: VMRef,
    context: ContextRef,
}

impl FiberInfo {
    pub fn new(vm: VMRef, context: ContextRef) -> Self {
        FiberInfo { vm, context }
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
    let mut new_vm = vm.clone();
    new_vm.clear_();
    new_vm.set_pc(0);
    new_vm.fiberstate_created();
    let val = Value::fiber(&vm.globals, VMRef::new(new_vm), context);
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
    let mut context = vm.context();
    let pc = vm.get_pc();
    let inst = &context.iseq_ref.iseq[pc];
    context.pc = pc + Inst::inst_size(*inst);
    vm.stack_push(Value::nil());
    Err(vm.error_fiber_yield(val))
}

// Instance methods

fn resume(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    match args.self_value.unpack() {
        RV::Object(obj) => match &obj.kind {
            ObjKind::Fiber(fiber) => {
                let context = fiber.context;
                let mut fiber_vm = fiber.vm;
                if fiber_vm.fiberstate() == FiberState::Dead {
                    return Err(vm.error_fiber("Dead fiber called."));
                }
                fiber_vm.fiberstate_running();
                match fiber_vm.vm_run_context(context) {
                    Ok(val) => {
                        fiber_vm.fiberstate_dead();
                        return Ok(val);
                    }
                    Err(err) => match err.kind {
                        RubyErrorKind::FiberYield(val) => return Ok(val),
                        _ => return Err(err),
                    },
                };
            }
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
}
