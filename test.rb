class B
    D = 999
end

class C
    D = 777
    class A < B
        p D
        p B::D
        D = 888
        p D
        p B::D
    end
end