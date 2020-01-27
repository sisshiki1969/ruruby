class A
    FOO = 100
    class B
        puts FOO
        FOO = 200
    end
    p FOO
end
puts A::FOO
puts A::B::FOO
p A.constants
p A::B.constants