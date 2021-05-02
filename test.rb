class C
    H = 1000
end
class A
    H = 100
    class B < C
        p H
        p defined? ::Object
    end
end