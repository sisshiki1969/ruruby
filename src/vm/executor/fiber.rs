use crate::coroutine::*;
use crate::*;

impl VM {
    pub fn dup_enum(&mut self, eref: &FiberContext, block: Option<Block>) -> Box<FiberContext> {
        match &eref.kind {
            FiberKind::Enum(box info) => {
                let mut info = info.clone();
                if let Some(block) = block {
                    info.args.block = Some(block)
                }
                Box::new(self.create_enum_info(info))
            }
            _ => unreachable!(),
        }
    }

    pub fn create_enumerator(
        &mut self,
        method: IdentId,
        receiver: Value,
        mut args: Args,
    ) -> VMResult {
        args.block = Some(self.new_block(METHOD_ENUM));
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
        let method = self_val.get_method_or_nomethod(method_name)?;
        //let context = ContextRef::new_native(self);
        //self.context_push(context);
        let val = self.eval_method(method, self_val, args)?;
        //self.context_pop();
        self.globals.error_register = val;
        Err(RubyError::stop_iteration("Iteration reached an end."))
    }

    fn create_enum_info(&mut self, info: EnumInfo) -> FiberContext {
        let fiber_vm = self.create_fiber();
        FiberContext::new_enumerator(fiber_vm, info)
    }
}
