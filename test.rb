class A
  def initialize
    @hclk = 0;
    @bg_pixels = [0,1,2,3,4,5,6]
    @bg_enabled = true
    @any_show = true
    @output_pixels = []
    @output_color = (0..1024).to_a
  end 
  def render_pixel
    if @any_show
      pixel = @bg_enabled ? @bg_pixels[@hclk % 8] : 0
      if @sp_active && (sprite = @sp_map[@hclk])
        if pixel % 4 == 0
          pixel = sprite[2]
        else
          @sp_zero_hit = true if sprite[1] && @hclk != 255
          pixel = sprite[2] unless sprite[0]
        end
      end
    else
      pixel = @scroll_addr_5_14 & 0x3f00 == 0x3f00 ? @scroll_addr_0_4 : 0
      @bg_pixels[@hclk % 8] = 0
    end
    @output_pixels << @output_color[pixel]
  end
end
