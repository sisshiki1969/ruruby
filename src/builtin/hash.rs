use crate::*;
use std::collections::HashMap;

pub fn init_hash(globals: &mut Globals) -> Value {
    let id = IdentId::get_ident_id("Hash");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "to_s", inspect);
    globals.add_builtin_instance_method(class, "inspect", inspect);
    globals.add_builtin_instance_method(class, "clear", clear);
    globals.add_builtin_instance_method(class, "clone", clone);
    globals.add_builtin_instance_method(class, "dup", clone);
    globals.add_builtin_instance_method(class, "compact", compact);
    globals.add_builtin_instance_method(class, "delete", delete);
    globals.add_builtin_instance_method(class, "empty?", empty);
    globals.add_builtin_instance_method(class, "select", select);
    globals.add_builtin_instance_method(class, "has_key?", has_key);
    globals.add_builtin_instance_method(class, "key?", has_key);
    globals.add_builtin_instance_method(class, "include?", has_key);
    globals.add_builtin_instance_method(class, "member?", has_key);
    globals.add_builtin_instance_method(class, "has_value?", has_value);
    globals.add_builtin_instance_method(class, "keys", keys);
    globals.add_builtin_instance_method(class, "length", length);
    globals.add_builtin_instance_method(class, "size", length);
    globals.add_builtin_instance_method(class, "values", values);
    globals.add_builtin_instance_method(class, "each_value", each_value);
    globals.add_builtin_instance_method(class, "each_key", each_key);
    globals.add_builtin_instance_method(class, "each", each);
    globals.add_builtin_instance_method(class, "merge", merge);
    globals.add_builtin_instance_method(class, "fetch", fetch);
    globals.add_builtin_instance_method(class, "compare_by_identity", compare_by_identity);
    globals.add_builtin_instance_method(class, "sort", sort);
    globals.add_builtin_instance_method(class, "invert", invert);
    Value::class(globals, class)
}

fn inspect(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    let s = hash.to_s(vm);
    Ok(Value::string(&vm.globals, s))
}

fn clear(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_mut_hash().unwrap();
    hash.clear();
    Ok(self_val)
}

fn clone(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    Ok(Value::hash_from(&vm.globals, (*hash).clone()))
}

fn compact(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let mut hash = self_val.expect_hash(vm, "Receiver")?.clone();
    match hash {
        HashInfo::Map(ref mut map) => map.retain(|_, &mut v| v != Value::nil()),
        HashInfo::IdentMap(ref mut map) => map.retain(|_, &mut v| v != Value::nil()),
    };
    Ok(Value::hash_from(&vm.globals, hash))
}

fn delete(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let hash = self_val.as_mut_hash().unwrap();
    let res = match hash.remove(args[0]) {
        Some(v) => v,
        None => Value::nil(),
    };
    Ok(res)
}

fn empty(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    Ok(Value::bool(hash.len() == 0))
}

fn select(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let hash = self_val.as_hash().unwrap();
    let method = vm.expect_block(args.block)?;
    let mut res = HashMap::new();
    let mut arg = Args::new2(Value::nil(), Value::nil());
    for (k, v) in hash.iter() {
        arg[0] = k;
        arg[1] = v;
        let b = vm.eval_block(method, &arg)?;
        if vm.val_to_bool(b) {
            res.insert(HashKey(k), v);
        };
    }

    Ok(Value::hash_from_map(&vm.globals, res))
}

fn has_key(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let hash = self_val.as_hash().unwrap();
    let res = hash.contains_key(args[0]);
    Ok(Value::bool(res))
}

fn has_value(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let hash = self_val.as_hash().unwrap();
    let res = hash.iter().find(|(_, v)| *v == args[0]).is_some();
    Ok(Value::bool(res))
}

fn length(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    let len = hash.len();
    Ok(Value::fixnum(len as i64))
}

fn keys(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    Ok(Value::array_from(&vm.globals, hash.keys()))
}

fn values(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    Ok(Value::array_from(&vm.globals, hash.values()))
}

fn each_value(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    let method = vm.expect_block(args.block)?;
    let mut arg = Args::new1(Value::nil());
    for (_, v) in hash.iter() {
        arg[0] = v;
        vm.eval_block(method, &arg)?;
    }

    Ok(self_val)
}

fn each_key(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    let method = vm.expect_block(args.block)?;
    let mut arg = Args::new1(Value::nil());

    for (k, _) in hash.iter() {
        arg[0] = k;
        vm.eval_block(method, &arg)?;
    }

    Ok(self_val)
}

fn each(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    let method = vm.expect_block(args.block)?;
    let mut arg = Args::new2(Value::nil(), Value::nil());

    for (k, v) in hash.iter() {
        arg[0] = k;
        arg[1] = v;
        vm.eval_block(method, &arg)?;
    }

    Ok(self_val)
}

fn merge(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let mut new = (self_val.expect_hash(vm, "Receiver")?).clone();
    for arg in args.iter() {
        let other = arg.expect_hash(vm, "First arg")?;
        for (k, v) in other.iter() {
            new.insert(k, v);
        }
    }

    Ok(Value::hash_from(&vm.globals, new))
}

fn fetch(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 1, 2)?;
    let key = args[0];

    let hash = self_val.as_hash().unwrap();
    let val = match hash.get(&key) {
        Some(val) => *val,
        None => {
            match args.block {
                // TODO: If arg[1] exists, Should warn "block supersedes default value argument".
                Some(block) => vm.eval_block(block, &Args::new1(key))?,
                None => {
                    if args.len() == 2 {
                        args[1]
                    } else {
                        // TODO: Should be KeyError.
                        return Err(vm.error_argument("Key not found."));
                    }
                }
            }
        }
    };

    Ok(val)
}

fn compare_by_identity(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_mut_hash().unwrap();
    match hash {
        HashInfo::Map(map) => {
            let new_map = map.into_iter().map(|(k, v)| (IdentKey(k.0), *v)).collect();
            *hash = HashInfo::IdentMap(new_map);
        }
        HashInfo::IdentMap(_) => {}
    };
    Ok(self_val)
}

fn sort(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    let mut vec = vec![];
    for (k, v) in hash.iter() {
        let ary = vec![k, v];
        vec.push(Value::array_from(&vm.globals, ary));
    }
    vm.sort_array(&mut vec)?;
    Ok(Value::array_from(&vm.globals, vec))
}

fn invert(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let hash = self_val.as_hash().unwrap();
    let mut new_hash = HashMap::new();
    for (k, v) in hash.iter() {
        new_hash.insert(HashKey(v), k);
    }
    Ok(Value::hash_from_map(&vm.globals, new_hash))
}

#[cfg(test)]
mod test {
    use crate::test::*;
    /*
        #[test]
        fn hash_inspect() {
            let program = r#"
                s = {:key=>"value", awesome: "Ruby"}.inspect
                assert("{:key=>\"value\", :awesome=>\"Ruby\"}", s)
            "#;
            assert_script(program);
        }
    */
    #[test]
    fn hash1() {
        let program = r#"
            h = {true => "true", false => "false", nil => "nil", 100 => "100", 7.7 => "7.7", "ruby" => "string", :ruby => "symbol"}
            assert(h[true], "true")
            assert(h[false], "false")
            assert(h[nil], "nil")
            assert(h[100], "100")
            assert(h[7.7], "7.7")
            assert(h["ruby"], "string")
            assert(h[:ruby], "symbol")
            assert([], h.keys - [true, false, nil, 100, 7.7, "ruby", :ruby])
            assert([], h.values - ["true", "false", "nil", "100", "7.7", "string", "symbol"])
        "#;
        assert_script(program);
    }

    #[test]
    fn hash2() {
        let program = r#"
            a = "100"
            @b = 7.7
            h = {true: "true", false: "false", nil: "nil", 100 => a, @b => "7.7", ruby: "string"}
            assert(h[:true], "true")
            assert(h[:false], "false")
            assert(h[:nil], "nil")
            assert(h[100], "100")
            assert(h[7.7], "7.7")
            assert(h[:ruby], "string")
            a = []
            h.each_key{|k| a << k}
            assert([], a - [:true, :false, :nil, 100, 7.7, :ruby])
            a = []
            h.each_value{|v| a << v}
            assert([], a - ["true", "false", "nil", "100", "7.7", "string"])
            a = []
            h.each{|k, v| a << [k, v];}
            assert([], a - [[:true, "true"], [:false, "false"], [:nil, "nil"], [100, "100"], [7.7, "7.7"], [:ruby, "string"]])
        "#;
        assert_script(program);
    }

    #[test]
    fn hash3() {
        let program = r#"
            h1 = {a: "symbol", c:nil, d:nil}
            assert(h1.has_key?(:a), true)
            assert(h1.has_key?(:b), false)
            assert(h1.has_value?("symbol"), true)
            assert(h1.has_value?(500), false)
            assert(h1.length, 3)
            assert(h1.size, 3)
            #assert(h1.keys, [:a, :d, :c])
            #assert(h1.values, ["symbol", nil, nil])
            h2 = h1.clone()
            h2[:b] = 100
            assert(h2[:b], 100)
            assert(h1[:b], nil)
            h3 = h2.compact
            assert(h3.delete(:a), "symbol")
            assert(h3.empty?, false)
            assert(h3.delete(:b), 100)
            assert(h3.delete(:c), nil)
            assert(h3.empty?, true)
            h2.clear()
            assert(h2.empty?, true)
        "#;
        assert_script(program);
    }

    #[test]
    fn hash_select() {
        let program = r#"
            h = { "a" => 100, "b" => 200, "c" => 300 }
            assert({"b" => 200, "c" => 300}, h.select {|k,v| k > "a"})  #=> {"b" => 200, "c" => 300}
            assert({"a" => 100}, h.select {|k,v| v < 200})  #=> {"a" => 100}
        "#;
        assert_script(program);
    }

    #[test]
    fn hash_merge() {
        let program = r#"
        h1 = { "a" => 100, "b" => 200 }
        h2 = { "b" => 246, "c" => 300 }
        h3 = { "b" => 357, "d" => 400 }
        assert({"a"=>100, "b"=>200}, h1.merge)
        assert({"a"=>100, "b"=>246, "c"=>300}, h1.merge(h2)) 
        assert({"a"=>100, "b"=>357, "c"=>300, "d"=>400}, h1.merge(h2, h3)) 
        assert({"a"=>100, "b"=>200}, h1)
    "#;
        assert_script(program);
    }

    #[test]
    fn hash_compare_by_identity() {
        let program = r#"
        a = "a"
        h1 = {}
        h1[a] = 100
        assert 100, h1["a"]
        assert 100, h1[a]
        h2 = {}
        h2.compare_by_identity
        h2[a] = 100
        assert nil, h2["a"]
        assert 100, h2[a]
    "#;
        assert_script(program);
    }

    #[test]
    fn hash_sort() {
        let program = r#"
        h = { 0 => 20, 1 => 30, 2 => 10  }
        assert([[0, 20], [1, 30], [2, 10]], h.sort)
        "#;
        assert_script(program);
    }

    #[test]
    fn hash_invert() {
        let program = r#"
        h = { "a" => 0, "b" => 100, "c" => 200, "e" => 300 }
        assert({0=>"a", 100=>"b", 200=>"c", 300=>"e"}, h.invert)
        "#;
        assert_script(program);
    }

    #[test]
    fn hash_fetch() {
        let program = r##"
            h = {one: nil}
            assert(nil, h[:one])                    #=> nil これではキーが存在するのか判別できない。
            assert(nil, h[:two])                    #=> nil これではキーが存在するのか判別できない。
            assert(nil, h.fetch(:one))
            assert_error { h.fetch(:two) }          # エラー key not found (KeyError)
            assert("error", h.fetch(:two,"error"))
            assert("two not exist", h.fetch(:two) {|key|"#{key} not exist"})
            res = h.fetch(:two, "error"){|key|
                "#{key} not exist"                  #  warning: block supersedes default value argument
            }        
            assert("two not exist", res)
            #h.default = "default"
            assert_error { h.fetch(:two) }          # エラー key not found (KeyError)
        "##;
        assert_script(program);
    }
}
