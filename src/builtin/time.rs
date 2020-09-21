use crate::*;
use chrono::{DateTime, Duration, FixedOffset, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct TimeInfo(DateTime<FixedOffset>);

pub fn init(_globals: &mut Globals) -> Value {
    let time_id = IdentId::get_id("Time");
    let mut class = ClassRef::from(time_id, BuiltinClass::object());
    let mut class_val = Value::class(class);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("-", sub);
    class.add_builtin_method_by_str("+", add);
    class_val.add_builtin_class_method("now", time_now);
    class_val
}

fn time_now(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let time_info = TimeInfo(Utc::now().with_timezone(&FixedOffset::east(9 * 3600)));
    let new_obj = Value::time(self_val, time_info);
    Ok(new_obj)
}

fn inspect(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let time = match &self_val.rvalue().kind {
        ObjKind::Time(time) => time.0,
        _ => unreachable!(),
    };
    Ok(Value::string(format!("{}", time)))
}

fn sub(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let time = match &self_val.rvalue().kind {
        ObjKind::Time(time) => time.0,
        _ => unreachable!(),
    };
    match args[0].unpack() {
        RV::Integer(i) => {
            let res = time - Duration::seconds(i);
            Ok(Value::time(self_val.get_class(), TimeInfo(res)))
        }
        RV::Float(f) => {
            let offset = (f * 1000.0 * 1000.0 * 1000.0) as i64;
            let res = time - Duration::nanoseconds(offset);
            Ok(Value::time(self_val.get_class(), TimeInfo(res)))
        }
        RV::Object(rv) => match &rv.kind {
            ObjKind::Time(t) => {
                let res = time - t.0;
                let offset = (res.num_nanoseconds().unwrap() as f64) / 1000.0 / 1000.0 / 1000.0;
                Ok(Value::float(offset))
            }
            _ => return Err(vm.error_undefined_op("-", args[0], self_val)),
        },
        _ => return Err(vm.error_undefined_op("-", args[0], self_val)),
    }
}

fn add(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let time = match &self_val.rvalue().kind {
        ObjKind::Time(time) => time.0,
        _ => unreachable!(),
    };
    match args[0].unpack() {
        RV::Integer(i) => {
            let res = time + Duration::seconds(i);
            Ok(Value::time(self_val.get_class(), TimeInfo(res)))
        }
        RV::Float(f) => {
            let offset = (f * 1000.0 * 1000.0 * 1000.0) as i64;
            let res = time + Duration::nanoseconds(offset);
            Ok(Value::time(self_val.get_class(), TimeInfo(res)))
        }
        _ => return Err(vm.error_undefined_op("+", args[0], self_val)),
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn time() {
        let program = "
        p Time.now.inspect
        a = Time.now
        assert a, a - 100 + 100
        assert a, a - 77.0 + 77.0
        assert Float, (Time.now - a).class
        assert_error { Time.now + a }
    ";
        assert_script(program);
    }
}
