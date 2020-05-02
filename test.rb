nmt_ref = [255] * 4096
entries = {}
lut_update = eval(File.read("lut_update.rb"))
TILE_LUT = eval(File.read("tile_lut.rb"))
(0..0x7fff).map do |i|
    io_addr = 0x23c0 | (i & 0x0c00) | (i >> 4 & 0x0038) | (i >> 2 & 0x0007)
    nmt_bank = nmt_ref[io_addr >> 10 & 3]
    nmt_idx = io_addr & 0x03ff
    attr_shift = (i & 2) | (i >> 4 & 4)
    key = [io_addr, attr_shift]
    entries[key] ||= [io_addr, TILE_LUT[nmt_bank[nmt_idx] >> attr_shift & 3], attr_shift]
    (((lut_update[nmt_bank] ||= [])[nmt_idx] ||= [nil, nil])[1] ||= []) << entries[key]
    entries[key]
end