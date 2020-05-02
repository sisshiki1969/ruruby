def lut
    eval(File.read("lut_update.rb"))
end

def tile
    eval(File.read("tile_lut.rb"))
end

@nmt_ref = [255] * 4096
@entries = {}
@lut_update = lut
TILE_LUT = tile
(0..0x7fff).map do |i|
    io_addr = 0x23c0 | (i & 0x0c00) | (i >> 4 & 0x0038) | (i >> 2 & 0x0007)
    nmt_bank = @nmt_ref[io_addr >> 10 & 3]
    nmt_idx = io_addr & 0x03ff
    attr_shift = (i & 2) | (i >> 4 & 4)
    key = [io_addr, attr_shift]
    @entries[key] ||= [io_addr, TILE_LUT[nmt_bank[nmt_idx] >> attr_shift & 3], attr_shift]
    b = @lut_update[nmt_bank] ||= []
    c = b[nmt_idx] ||= [nil, nil]
    a = c[1] ||= []
    a << @entries[key]
    @entries[key]
end


