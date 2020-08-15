use crate::*;
use chrono::{DateTime, Duration, FixedOffset, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct TimeInfo(DateTime<FixedOffset>);

pub fn init_time(globals: &mut Globals) -> Value {
    let time_id = IdentId::get_id("Time");
    let class = ClassRef::from(time_id, globals.builtins.object);
    let class_obj = Value::class(globals, class);
    globals.add_builtin_instance_method(class, "inspect", inspect);
    globals.add_builtin_instance_method(class, "-", sub);
    globals.add_builtin_instance_method(class, "+", add);
    globals.add_builtin_class_method(class_obj, "now", time_now);
    class_obj
}

fn time_now(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 0)?;
    let time_info = TimeInfo(Utc::now().with_timezone(&FixedOffset::east(9 * 3600)));
    let new_obj = Value::time(self_val, time_info);
    Ok(new_obj)
}

fn inspect(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 0)?;
    let time = match &self_val.rvalue().kind {
        ObjKind::Time(time) => time.0,
        _ => unreachable!(),
    };
    Ok(Value::string(&vm.globals.builtins, format!("{}", time)))
}

fn sub(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 1)?;
    let time = match &self_val.rvalue().kind {
        ObjKind::Time(time) => time.0,
        _ => unreachable!(),
    };
    match args[0].unpack() {
        RV::Integer(i) => {
            let res = time - Duration::seconds(i);
            Ok(Value::time(
                self_val.get_class_object(&vm.globals),
                TimeInfo(res),
            ))
        }
        RV::Float(f) => {
            let offset = (f * 1000.0 * 1000.0 * 1000.0) as i64;
            let res = time - Duration::nanoseconds(offset);
            Ok(Value::time(
                self_val.get_class_object(&vm.globals),
                TimeInfo(res),
            ))
        }
        RV::Object(rv) => match &rv.kind {
            ObjKind::Time(t) => {
                let res = time - t.0;
                let offset = (res.num_nanoseconds().unwrap() as f64) / 1000.0 / 1000.0 / 1000.0;
                Ok(Value::flonum(offset))
            }
            _ => return Err(vm.error_undefined_op("-", args[0], self_val)),
        },
        _ => return Err(vm.error_undefined_op("-", args[0], self_val)),
    }
}

fn add(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 1)?;
    let time = match &self_val.rvalue().kind {
        ObjKind::Time(time) => time.0,
        _ => unreachable!(),
    };
    match args[0].unpack() {
        RV::Integer(i) => {
            let res = time + Duration::seconds(i);
            Ok(Value::time(
                self_val.get_class_object(&vm.globals),
                TimeInfo(res),
            ))
        }
        RV::Float(f) => {
            let offset = (f * 1000.0 * 1000.0 * 1000.0) as i64;
            let res = time + Duration::nanoseconds(offset);
            Ok(Value::time(
                self_val.get_class_object(&vm.globals),
                TimeInfo(res),
            ))
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
