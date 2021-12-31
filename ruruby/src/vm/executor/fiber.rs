use crate::coroutine::*;
use crate::*;

impl VM {
    pub(crate) fn dup_enum(&mut self, eref: &FiberContext) -> Box<FiberContext> {
        match &eref.kind {
            FiberKind::Enum(box info) => Box::new(self.create_enum_info(info.clone())),
            _ => unreachable!(),
        }
    }

    /// Create Enumerator object.
    /// This fn is to be called from class library.
    pub(crate) fn create_enumerator(
        &mut self,
        method: IdentId,
        receiver: Value,
        mut args: Args,
    ) -> VMResult {
        let outer = self.caller_cfp();
        let self_val = outer.self_value();
        let proc = Value::procobj(self, self_val, METHOD_ENUM, outer);

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
    pub(crate) fn enumerator_fiber(
        &mut self,
        self_val: Value,
        args: &Args,
        method_name: IdentId,
    ) -> VMResult {
        let method = self_val.get_method_or_nomethod(&mut self.globals, method_name)?;
        let val = self.eval_method(method, self_val, &args, &Args2::from(args))?;
        self.globals.val = val;
        Err(RubyError::stop_iteration("Iteration reached an end."))
    }

    fn create_enum_info(&mut self, info: EnumInfo) -> FiberContext {
        let fiber_vm = self.create_fiber();
        FiberContext::new_enumerator(fiber_vm, info)
    }
}
