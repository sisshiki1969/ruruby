#![feature(test)]
extern crate ruruby;
extern crate test;
use ruruby::test::*;

#[test]
fn array() {
    let program = "
    a=[1,2,3,4]
    assert(3, a[2]);
    a[1] = 14
    assert(a, [1,14,3,4])
    a.pop()
    assert(a, [1,14,3])
    a.push(7,8,9)
    assert(a, [1,14,3,7,8,9])
    a=[1,2,3,4]
    b=Array.new(a)
    assert(a,b)
    b[2] = 100
    assert(a, [1,2,3,4])
    assert(b, [1,2,100,4])
    ";
    assert_script(program);
}

#[test]
fn array1() {
    let program = "
    assert([1,2,3]*0, [])
    assert([1,2,3]*1, [1,2,3])
    assert([nil]*5, [nil,nil,nil,nil,nil])
    assert([1,2,3]+[3,4,5], [1,2,3,3,4,5])
    assert([1,2,3]-[3,4,5], [1,2])
    ";
    assert_script(program);
}

#[test]
fn array2() {
    let program = "
    a = [1,2,3,4,5,6,7]
    b = [3,9]
    c = [3,3]
    assert(a[2], 3)
    assert(a[3,9], [4,5,6,7])
    assert(a[*b], [4,5,6,7])
    assert(a[3,3], [4,5,6])
    assert(a[*c], [4,5,6])
    assert(a[7], nil)
    assert(a[7,3], [])
    ";
    assert_script(program);
}

#[test]
fn array3() {
    let program = "
    a = [1,2,3,4,5,6,7]
    assert(a[2,3], [3,4,5])
    a[2,3] = 100
    assert(a, [1,2,100,6,7])
    ";
    assert_script(program);
}

#[test]
fn array_push() {
    let program = r#"
    a = [1,2,3]
    a << 4
    a << "Ruby"
    assert([1,2,3,4,"Ruby"], a)
    "#;
    assert_script(program);
}

#[test]
fn array_map() {
    let program = "
    a = [1,2,3]
    assert(a.map {|| 3 }, [3,3,3])
    assert(a.map {|x| x*3 }, [3,6,9])
    assert(a.map do |x| x*3 end, [3,6,9])
    assert(a, [1,2,3])";
    assert_script(program);
}

#[test]
fn array_include() {
    let program = r#"
    a = ["ruby","rust","java"]
    assert(true, a.include?("ruby"))
    assert(true, a.include?("rust"))
    assert(false, a.include?("c++"))
    assert(false, a.include?(:ruby))
    "#;
    assert_script(program);
}

#[test]
fn array_each() {
    let program = "
    a = [1,2,3]
    b = 0
    assert([1,2,3], a.each {|x| b+=x })
    assert(6, b)
    ";
    assert_script(program);
}

#[test]
fn array_reverse() {
    let program = "
    a = [1,2,3,4,5]
    assert([5,4,3,2,1], a.reverse)
    assert([1,2,3,4,5], a)
    assert([5,4,3,2,1], a.reverse!)
    assert([5,4,3,2,1], a)
    ";
    assert_script(program);
}
