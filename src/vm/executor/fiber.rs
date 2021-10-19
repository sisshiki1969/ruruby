use crate::coroutine::*;
use crate::*;

impl VM {
    pub fn dup_enum(&mut self, eref: &FiberContext, block: Option<Block>) -> Box<FiberContext> {
        match &eref.kind {
            FiberKind::Enum(box info) => {
                let mut info = info.clone();
                if let Some(block) = block {
                    let p = self.create_proc(&block);
                    // This is necessary for GC.
                    self.temp_push(p);
                    info.args.block = Some(p.into());
                }
                Box::new(self.create_enum_info(info))
            }
            _ => unreachable!(),
        }
    }

    /// Create Enumerator object.
    /// This fn is to be called from class library.
    pub fn create_enumerator(
        &mut self,
        method: IdentId,
        receiver: Value,
        mut args: Args,
    ) -> VMResult {
        let proc = self.create_proc_from_block(METHOD_ENUM, self.cur_outer_frame());
        args.block = Some(proc.into());
        let fiber = self.create_enum_info(EnumInfo {
            method,
            receiver,
            args,
        });
        Ok(Value::enumerator(fiber))
    }

    /// This func is called in the fiber thread of a enumerator.
    /// `vm`: VM of created fiber.
    pub fn enumerator_fiber(
        &mut self,
        self_val: Value,
        args: &Args,
        method_name: IdentId,
    ) -> VMResult {
        let method = self_val.get_method_or_nomethod(&mut self.globals, method_name)?;
        let val = self.eval_method(method, self_val, args)?;
        self.globals.val = val;
        Err(RubyError::stop_iteration("Iteration reached an end."))
    }

    fn create_enum_info(&mut self, info: EnumInfo) -> FiberContext {
        let fiber_vm = self.create_fiber();
        FiberContext::new_enumerator(fiber_vm, info)
    }
}
