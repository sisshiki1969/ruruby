class A
    def foo(a,b,c,d:0)
        puts "output of A(super) #{a},#{b},#{c},#{d}"
    end            
 end

class B < A
    def foo(a,b,c=300,d:400)
        super(a,b,c,d:d)
        puts "output of B #{a},#{b},#{c},#{d}"
    end
end

B.new.foo(100,200,d:500)