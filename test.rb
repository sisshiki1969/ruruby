def lut
    eval(File.read("lut_update.rb"))
end

def tile
    eval(File.read("tile_lut.rb"))
end

a = []
b = 0
while b < 10000 do
    a << 0
    b += 1
end

