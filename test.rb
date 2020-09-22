class A < Array
    def foo
        100
    end
end

a = A.new
raise unless A == a.class
raise unless Array == a.class.superclass
raise unless 100 == a.foo