def assert(expected, actual)
  if expected == actual
    puts "OK #{actual}"
  else
    puts "NG expected: #{expected} actual: #{actual}"
  end
end

class A
  def foo(a,b,c,d:0)
      assert [100,200,300,500], [a,b,c,d]
  end 
  def boo(*a)
      assert [100,200,300], a
  end
  def bee(a:1,b:2,c:3)
      assert [1,2,3], [a,b,c]
  end
end            

class B < A
  def foo(a,b,c=300,d:400)
      super(a,b,c,d:d)
  end
  def boo(a,b,c)
      super
  end
  def bee(a,b,c)
      super()
  end
end

B.new.foo(100,200,d:500)
B.new.boo(100,200,300)
B.new.bee(100,200,300)
