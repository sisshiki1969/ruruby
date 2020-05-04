def self.run
  begin
    __a12_monitor__ = @a12_monitor
    __a12_state__ = @a12_state
    __any_show__ = @any_show
    __attr_lut__ = @attr_lut
    __bg_enabled__ = @bg_enabled
    __bg_pattern__ = @bg_pattern
    __bg_pattern_base__ = @bg_pattern_base
    __bg_pattern_base_15__ = @bg_pattern_base_15
    __bg_pattern_lut__ = @bg_pattern_lut
    __bg_pattern_lut_fetched__ = @bg_pattern_lut_fetched
    __bg_pixels__ = @bg_pixels
    __bg_show__ = @bg_show
    __bg_show_edge__ = @bg_show_edge
    __chr_mem__ = @chr_mem
    __cpu__ = @cpu
    __hclk__ = @hclk
    __hclk_target__ = @hclk_target
    __io_addr__ = @io_addr
    __io_pattern__ = @io_pattern
    __name_io_addr__ = @name_io_addr
    __name_lut__ = @name_lut
    __need_nmi__ = @need_nmi
    __nmt_ref__ = @nmt_ref
    __odd_frame__ = @odd_frame
    __output_color__ = @output_color
    __output_pixels__ = @output_pixels
    __pattern_end__ = @pattern_end
    __regs_oam__ = @regs_oam
    __scanline__ = @scanline
    __scroll_addr_0_4__ = @scroll_addr_0_4
    __scroll_addr_5_14__ = @scroll_addr_5_14
    __scroll_latch__ = @scroll_latch
    __scroll_xfine__ = @scroll_xfine
    __sp_active__ = @sp_active
    __sp_addr__ = @sp_addr
    __sp_base__ = @sp_base
    __sp_buffer__ = @sp_buffer
    __sp_buffered__ = @sp_buffered
    __sp_enabled__ = @sp_enabled
    __sp_height__ = @sp_height
    __sp_index__ = @sp_index
    __sp_latch__ = @sp_latch
    __sp_limit__ = @sp_limit
    __sp_map__ = @sp_map
    __sp_map_buffer__ = @sp_map_buffer
    __sp_overflow__ = @sp_overflow
    __sp_phase__ = @sp_phase
    __sp_ram__ = @sp_ram
    __sp_show__ = @sp_show
    __sp_show_edge__ = @sp_show_edge
    __sp_visible__ = @sp_visible
    __sp_zero_hit__ = @sp_zero_hit
    __sp_zero_in_line__ = @sp_zero_in_line
    __vblank__ = @vblank
    __vblanking__ = @vblanking
    __vclk__ = @vclk
    __name_io_addr__ = (__scroll_addr_0_4__ | __scroll_addr_5_14__) & 0x0fff | 0x2000
    __bg_pattern_lut_fetched__ = TILE_LUT[
      __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >> ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3
    ]
    if __any_show__
      if __a12_monitor__
        while __hclk_target__ > __hclk__
          case __hclk__
          when 0, 8, 16, 24, 32, 40, 48, 56
            if __hclk__ + 8 <= __hclk_target__
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              # batch-version of render_pixel
              if __sp_active__
                if __bg_enabled__
                  pixel0 = __bg_pixels__[0]
                  if sprite = __sp_map__[__hclk__]
                    if pixel0 % 4 == 0
                      pixel0 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel0 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel1 = __bg_pixels__[1]
                  if sprite = __sp_map__[__hclk__ + 1]
                    if pixel1 % 4 == 0
                      pixel1 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel1 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel2 = __bg_pixels__[2]
                  if sprite = __sp_map__[__hclk__ + 2]
                    if pixel2 % 4 == 0
                      pixel2 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel2 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel3 = __bg_pixels__[3]
                  if sprite = __sp_map__[__hclk__ + 3]
                    if pixel3 % 4 == 0
                      pixel3 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel3 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel4 = __bg_pixels__[4]
                  if sprite = __sp_map__[__hclk__ + 4]
                    if pixel4 % 4 == 0
                      pixel4 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel4 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel5 = __bg_pixels__[5]
                  if sprite = __sp_map__[__hclk__ + 5]
                    if pixel5 % 4 == 0
                      pixel5 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel5 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel6 = __bg_pixels__[6]
                  if sprite = __sp_map__[__hclk__ + 6]
                    if pixel6 % 4 == 0
                      pixel6 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel6 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel7 = __bg_pixels__[7]
                  if sprite = __sp_map__[__hclk__ + 7]
                    if pixel7 % 4 == 0
                      pixel7 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel7 = sprite[2] unless sprite[0]
                    end
                  end
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                else
                  pixel0 = (sprite = __sp_map__[__hclk__ ]) ? sprite[2] : 0
                  pixel1 = (sprite = __sp_map__[__hclk__  + 1]) ? sprite[2] : 0
                  pixel2 = (sprite = __sp_map__[__hclk__  + 2]) ? sprite[2] : 0
                  pixel3 = (sprite = __sp_map__[__hclk__  + 3]) ? sprite[2] : 0
                  pixel4 = (sprite = __sp_map__[__hclk__  + 4]) ? sprite[2] : 0
                  pixel5 = (sprite = __sp_map__[__hclk__  + 5]) ? sprite[2] : 0
                  pixel6 = (sprite = __sp_map__[__hclk__  + 6]) ? sprite[2] : 0
                  pixel7 = (sprite = __sp_map__[__hclk__  + 7]) ? sprite[2] : 0
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                end
              else
                if __bg_enabled__ # this is the true hot-spot
                  __output_pixels__ << __output_color__[__bg_pixels__[0]] << __output_color__[__bg_pixels__[1]] << __output_color__[__bg_pixels__[2]] << __output_color__[__bg_pixels__[3]] << __output_color__[__bg_pixels__[4]] << __output_color__[__bg_pixels__[5]] << __output_color__[__bg_pixels__[6]] << __output_color__[__bg_pixels__[7]]
                else
                  clr = __output_color__[0]
                  __output_pixels__ << clr << clr << clr << clr << clr << clr << clr << clr
                end
              end
              __io_addr__ = __name_io_addr__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]
              __hclk__ += 1
              __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern_lut__ = __bg_pattern_lut_fetched__
              # raise unless __bg_pattern_lut_fetched__ ==
              #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
              #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3
              if __scroll_addr_0_4__ < 0x001f
                __scroll_addr_0_4__ += 1
                __name_io_addr__ += 1 # make cache consistent
              else
                __scroll_addr_0_4__ = 0
                __scroll_addr_5_14__ ^= 0x0400
                __name_io_addr__ ^= 0x041f # make cache consistent
              end
              __hclk__ += 1
              __io_addr__ = __io_pattern__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]
              __hclk__ += 1
              __io_addr__ = __io_pattern__ | 8
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100
              # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __bg_enabled__ = __bg_show__
              __sp_enabled__ = __sp_show__
              __sp_active__ = __sp_enabled__ && __sp_visible__
              # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __hclk__ += 1
            else
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              __io_addr__ = __name_io_addr__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
              if __sp_active__ && (sprite = __sp_map__[__hclk__])
                if pixel % 4 == 0
                  pixel = sprite[2]
                else
                  if sprite[1] && __hclk__ != 255
                    __sp_zero_hit__ = true
                  end
                  unless sprite[0]
                    pixel = sprite[2]
                  end
                end
              end
              __output_pixels__ << __output_color__[pixel]
              __hclk__ += 1
            end
          when 1, 9, 17, 25, 33, 41, 49, 57
            __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 2, 10, 18, 26, 34, 42, 50, 58
            __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 3, 11, 19, 27, 35, 43, 51, 59
            __bg_pattern_lut__ = __bg_pattern_lut_fetched__
            # raise unless __bg_pattern_lut_fetched__ ==
            #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
            #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3
            if __scroll_addr_0_4__ < 0x001f
              __scroll_addr_0_4__ += 1
              __name_io_addr__ += 1 # make cache consistent
            else
              __scroll_addr_0_4__ = 0
              __scroll_addr_5_14__ ^= 0x0400
              __name_io_addr__ ^= 0x041f # make cache consistent
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 4, 12, 20, 28, 36, 44, 52, 60
            __io_addr__ = __io_pattern__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 5, 13, 21, 29, 37, 45, 53, 61
            __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 6, 14, 22, 30, 38, 46, 54, 62
            __io_addr__ = __io_pattern__ | 8
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 7, 15, 23, 31, 39, 47, 55, 63
            __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __bg_enabled__ = __bg_show__
            __sp_enabled__ = __sp_show__
            __sp_active__ = __sp_enabled__ && __sp_visible__
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          when 64
            if __hclk__ + 8 <= __hclk_target__
              __sp_addr__ = __regs_oam__ & 0xf8 # SP_OFFSET_TO_0_1
              __sp_phase__ = nil
              __sp_latch__ = 0xff
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              # batch-version of render_pixel
              if __sp_active__
                if __bg_enabled__
                  pixel0 = __bg_pixels__[0]
                  if sprite = __sp_map__[__hclk__]
                    if pixel0 % 4 == 0
                      pixel0 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel0 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel1 = __bg_pixels__[1]
                  if sprite = __sp_map__[__hclk__ + 1]
                    if pixel1 % 4 == 0
                      pixel1 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel1 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel2 = __bg_pixels__[2]
                  if sprite = __sp_map__[__hclk__ + 2]
                    if pixel2 % 4 == 0
                      pixel2 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel2 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel3 = __bg_pixels__[3]
                  if sprite = __sp_map__[__hclk__ + 3]
                    if pixel3 % 4 == 0
                      pixel3 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel3 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel4 = __bg_pixels__[4]
                  if sprite = __sp_map__[__hclk__ + 4]
                    if pixel4 % 4 == 0
                      pixel4 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel4 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel5 = __bg_pixels__[5]
                  if sprite = __sp_map__[__hclk__ + 5]
                    if pixel5 % 4 == 0
                      pixel5 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel5 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel6 = __bg_pixels__[6]
                  if sprite = __sp_map__[__hclk__ + 6]
                    if pixel6 % 4 == 0
                      pixel6 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel6 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel7 = __bg_pixels__[7]
                  if sprite = __sp_map__[__hclk__ + 7]
                    if pixel7 % 4 == 0
                      pixel7 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel7 = sprite[2] unless sprite[0]
                    end
                  end
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                else
                  pixel0 = (sprite = __sp_map__[__hclk__ ]) ? sprite[2] : 0
                  pixel1 = (sprite = __sp_map__[__hclk__  + 1]) ? sprite[2] : 0
                  pixel2 = (sprite = __sp_map__[__hclk__  + 2]) ? sprite[2] : 0
                  pixel3 = (sprite = __sp_map__[__hclk__  + 3]) ? sprite[2] : 0
                  pixel4 = (sprite = __sp_map__[__hclk__  + 4]) ? sprite[2] : 0
                  pixel5 = (sprite = __sp_map__[__hclk__  + 5]) ? sprite[2] : 0
                  pixel6 = (sprite = __sp_map__[__hclk__  + 6]) ? sprite[2] : 0
                  pixel7 = (sprite = __sp_map__[__hclk__  + 7]) ? sprite[2] : 0
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                end
              else
                if __bg_enabled__ # this is the true hot-spot
                  __output_pixels__ << __output_color__[__bg_pixels__[0]] << __output_color__[__bg_pixels__[1]] << __output_color__[__bg_pixels__[2]] << __output_color__[__bg_pixels__[3]] << __output_color__[__bg_pixels__[4]] << __output_color__[__bg_pixels__[5]] << __output_color__[__bg_pixels__[6]] << __output_color__[__bg_pixels__[7]]
                else
                  clr = __output_color__[0]
                  __output_pixels__ << clr << clr << clr << clr << clr << clr << clr << clr
                end
              end
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern_lut__ = __bg_pattern_lut_fetched__
              # raise unless __bg_pattern_lut_fetched__ ==
              #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
              #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              if __scroll_addr_0_4__ < 0x001f
                __scroll_addr_0_4__ += 1
                __name_io_addr__ += 1 # make cache consistent
              else
                __scroll_addr_0_4__ = 0
                __scroll_addr_5_14__ ^= 0x0400
                __name_io_addr__ ^= 0x041f # make cache consistent
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__ | 8
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __bg_enabled__ = __bg_show__
              __sp_enabled__ = __sp_show__
              __sp_active__ = __sp_enabled__ && __sp_visible__
              # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __hclk__ += 1
            else
              __sp_addr__ = __regs_oam__ & 0xf8 # SP_OFFSET_TO_0_1
              __sp_phase__ = nil
              __sp_latch__ = 0xff
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
              if __sp_active__ && (sprite = __sp_map__[__hclk__])
                if pixel % 4 == 0
                  pixel = sprite[2]
                else
                  if sprite[1] && __hclk__ != 255
                    __sp_zero_hit__ = true
                  end
                  unless sprite[0]
                    pixel = sprite[2]
                  end
                end
              end
              __output_pixels__ << __output_color__[pixel]
              __hclk__ += 1
            end
          when 65, 73, 81, 89, 97, 105, 113, 121, 129, 137, 145, 153, 161, 169, 177, 185, 193, 201, 209, 217, 225, 233, 241, 249
            __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 66, 74, 82, 90, 98, 106, 114, 122, 130, 138, 146, 154, 162, 170, 178, 186, 194, 202, 210, 218, 226, 234, 242, 250
            __sp_latch__ = __sp_ram__[__sp_addr__]
            __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 67, 75, 83, 91, 99, 107, 115, 123, 131, 139, 147, 155, 163, 171, 179, 187, 195, 203, 211, 219, 227, 235, 243
            __bg_pattern_lut__ = __bg_pattern_lut_fetched__
            # raise unless __bg_pattern_lut_fetched__ ==
            #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
            #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            if __scroll_addr_0_4__ < 0x001f
              __scroll_addr_0_4__ += 1
              __name_io_addr__ += 1 # make cache consistent
            else
              __scroll_addr_0_4__ = 0
              __scroll_addr_5_14__ ^= 0x0400
              __name_io_addr__ ^= 0x041f # make cache consistent
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 68, 76, 84, 92, 100, 108, 116, 124, 132, 140, 148, 156, 164, 172, 180, 188, 196, 204, 212, 220, 228, 236, 244, 252
            __sp_latch__ = __sp_ram__[__sp_addr__]
            __io_addr__ = __io_pattern__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 69, 77, 85, 93, 101, 109, 117, 125, 133, 141, 149, 157, 165, 173, 181, 189, 197, 205, 213, 221, 229, 237, 245, 253
            __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 70, 78, 86, 94, 102, 110, 118, 126, 134, 142, 150, 158, 166, 174, 182, 190, 198, 206, 214, 222, 230, 238, 246, 254
            __sp_latch__ = __sp_ram__[__sp_addr__]
            __io_addr__ = __io_pattern__ | 8
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 71, 79, 87, 95, 103, 111, 119, 127, 135, 143, 151, 159, 167, 175, 183, 191, 199, 207, 215, 223, 231, 239, 247
            __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __bg_enabled__ = __bg_show__
            __sp_enabled__ = __sp_show__
            __sp_active__ = __sp_enabled__ && __sp_visible__
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          when 72, 80, 88, 96, 104, 112, 120, 128, 136, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216, 224, 232, 240
            if __hclk__ + 8 <= __hclk_target__
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              # batch-version of render_pixel
              if __sp_active__
                if __bg_enabled__
                  pixel0 = __bg_pixels__[0]
                  if sprite = __sp_map__[__hclk__]
                    if pixel0 % 4 == 0
                      pixel0 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel0 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel1 = __bg_pixels__[1]
                  if sprite = __sp_map__[__hclk__ + 1]
                    if pixel1 % 4 == 0
                      pixel1 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel1 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel2 = __bg_pixels__[2]
                  if sprite = __sp_map__[__hclk__ + 2]
                    if pixel2 % 4 == 0
                      pixel2 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel2 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel3 = __bg_pixels__[3]
                  if sprite = __sp_map__[__hclk__ + 3]
                    if pixel3 % 4 == 0
                      pixel3 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel3 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel4 = __bg_pixels__[4]
                  if sprite = __sp_map__[__hclk__ + 4]
                    if pixel4 % 4 == 0
                      pixel4 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel4 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel5 = __bg_pixels__[5]
                  if sprite = __sp_map__[__hclk__ + 5]
                    if pixel5 % 4 == 0
                      pixel5 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel5 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel6 = __bg_pixels__[6]
                  if sprite = __sp_map__[__hclk__ + 6]
                    if pixel6 % 4 == 0
                      pixel6 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel6 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel7 = __bg_pixels__[7]
                  if sprite = __sp_map__[__hclk__ + 7]
                    if pixel7 % 4 == 0
                      pixel7 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel7 = sprite[2] unless sprite[0]
                    end
                  end
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                else
                  pixel0 = (sprite = __sp_map__[__hclk__ ]) ? sprite[2] : 0
                  pixel1 = (sprite = __sp_map__[__hclk__  + 1]) ? sprite[2] : 0
                  pixel2 = (sprite = __sp_map__[__hclk__  + 2]) ? sprite[2] : 0
                  pixel3 = (sprite = __sp_map__[__hclk__  + 3]) ? sprite[2] : 0
                  pixel4 = (sprite = __sp_map__[__hclk__  + 4]) ? sprite[2] : 0
                  pixel5 = (sprite = __sp_map__[__hclk__  + 5]) ? sprite[2] : 0
                  pixel6 = (sprite = __sp_map__[__hclk__  + 6]) ? sprite[2] : 0
                  pixel7 = (sprite = __sp_map__[__hclk__  + 7]) ? sprite[2] : 0
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                end
              else
                if __bg_enabled__ # this is the true hot-spot
                  __output_pixels__ << __output_color__[__bg_pixels__[0]] << __output_color__[__bg_pixels__[1]] << __output_color__[__bg_pixels__[2]] << __output_color__[__bg_pixels__[3]] << __output_color__[__bg_pixels__[4]] << __output_color__[__bg_pixels__[5]] << __output_color__[__bg_pixels__[6]] << __output_color__[__bg_pixels__[7]]
                else
                  clr = __output_color__[0]
                  __output_pixels__ << clr << clr << clr << clr << clr << clr << clr << clr
                end
              end
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern_lut__ = __bg_pattern_lut_fetched__
              # raise unless __bg_pattern_lut_fetched__ ==
              #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
              #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              if __scroll_addr_0_4__ < 0x001f
                __scroll_addr_0_4__ += 1
                __name_io_addr__ += 1 # make cache consistent
              else
                __scroll_addr_0_4__ = 0
                __scroll_addr_5_14__ ^= 0x0400
                __name_io_addr__ ^= 0x041f # make cache consistent
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__ | 8
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __bg_enabled__ = __bg_show__
              __sp_enabled__ = __sp_show__
              __sp_active__ = __sp_enabled__ && __sp_visible__
              # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __hclk__ += 1
            else
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
              if __sp_active__ && (sprite = __sp_map__[__hclk__])
                if pixel % 4 == 0
                  pixel = sprite[2]
                else
                  if sprite[1] && __hclk__ != 255
                    __sp_zero_hit__ = true
                  end
                  unless sprite[0]
                    pixel = sprite[2]
                  end
                end
              end
              __output_pixels__ << __output_color__[pixel]
              __hclk__ += 1
            end
          when 248
            if __hclk__ + 8 <= __hclk_target__
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              # batch-version of render_pixel
              if __sp_active__
                if __bg_enabled__
                  pixel0 = __bg_pixels__[0]
                  if sprite = __sp_map__[__hclk__]
                    if pixel0 % 4 == 0
                      pixel0 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel0 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel1 = __bg_pixels__[1]
                  if sprite = __sp_map__[__hclk__ + 1]
                    if pixel1 % 4 == 0
                      pixel1 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel1 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel2 = __bg_pixels__[2]
                  if sprite = __sp_map__[__hclk__ + 2]
                    if pixel2 % 4 == 0
                      pixel2 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel2 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel3 = __bg_pixels__[3]
                  if sprite = __sp_map__[__hclk__ + 3]
                    if pixel3 % 4 == 0
                      pixel3 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel3 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel4 = __bg_pixels__[4]
                  if sprite = __sp_map__[__hclk__ + 4]
                    if pixel4 % 4 == 0
                      pixel4 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel4 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel5 = __bg_pixels__[5]
                  if sprite = __sp_map__[__hclk__ + 5]
                    if pixel5 % 4 == 0
                      pixel5 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel5 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel6 = __bg_pixels__[6]
                  if sprite = __sp_map__[__hclk__ + 6]
                    if pixel6 % 4 == 0
                      pixel6 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel6 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel7 = __bg_pixels__[7]
                  if sprite = __sp_map__[__hclk__ + 7]
                    if pixel7 % 4 == 0
                      pixel7 = sprite[2]
                    else
                      pixel7 = sprite[2] unless sprite[0]
                    end
                  end
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                else
                  pixel0 = (sprite = __sp_map__[__hclk__ ]) ? sprite[2] : 0
                  pixel1 = (sprite = __sp_map__[__hclk__  + 1]) ? sprite[2] : 0
                  pixel2 = (sprite = __sp_map__[__hclk__  + 2]) ? sprite[2] : 0
                  pixel3 = (sprite = __sp_map__[__hclk__  + 3]) ? sprite[2] : 0
                  pixel4 = (sprite = __sp_map__[__hclk__  + 4]) ? sprite[2] : 0
                  pixel5 = (sprite = __sp_map__[__hclk__  + 5]) ? sprite[2] : 0
                  pixel6 = (sprite = __sp_map__[__hclk__  + 6]) ? sprite[2] : 0
                  pixel7 = (sprite = __sp_map__[__hclk__  + 7]) ? sprite[2] : 0
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                end
              else
                if __bg_enabled__ # this is the true hot-spot
                  __output_pixels__ << __output_color__[__bg_pixels__[0]] << __output_color__[__bg_pixels__[1]] << __output_color__[__bg_pixels__[2]] << __output_color__[__bg_pixels__[3]] << __output_color__[__bg_pixels__[4]] << __output_color__[__bg_pixels__[5]] << __output_color__[__bg_pixels__[6]] << __output_color__[__bg_pixels__[7]]
                else
                  clr = __output_color__[0]
                  __output_pixels__ << clr << clr << clr << clr << clr << clr << clr << clr
                end
              end
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern_lut__ = __bg_pattern_lut_fetched__
              # raise unless __bg_pattern_lut_fetched__ ==
              #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
              #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              if __scroll_addr_5_14__ & 0x7000 != 0x7000
                __scroll_addr_5_14__ += 0x1000
              else
                mask = __scroll_addr_5_14__ & 0x03e0
                if mask == 0x03a0
                  __scroll_addr_5_14__ ^= 0x0800
                  __scroll_addr_5_14__ &= 0x0c00
                elsif mask == 0x03e0
                  __scroll_addr_5_14__ &= 0x0c00
                else
                  __scroll_addr_5_14__ = (__scroll_addr_5_14__ & 0x0fe0) + 32
                end
              end

              __name_io_addr__ = (__scroll_addr_0_4__ | __scroll_addr_5_14__) & 0x0fff | 0x2000 # make cache consistent
              if __scroll_addr_0_4__ < 0x001f
                __scroll_addr_0_4__ += 1
                __name_io_addr__ += 1 # make cache consistent
              else
                __scroll_addr_0_4__ = 0
                __scroll_addr_5_14__ ^= 0x0400
                __name_io_addr__ ^= 0x041f # make cache consistent
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__ | 8
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              __hclk__ += 1
              __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __hclk__ += 1
            else
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              a12_state = __io_addr__[12] == 1
              if !__a12_state__ && a12_state
                __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
              end
              __a12_state__ = a12_state
              pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
              if __sp_active__ && (sprite = __sp_map__[__hclk__])
                if pixel % 4 == 0
                  pixel = sprite[2]
                else
                  if sprite[1] && __hclk__ != 255
                    __sp_zero_hit__ = true
                  end
                  unless sprite[0]
                    pixel = sprite[2]
                  end
                end
              end
              __output_pixels__ << __output_color__[pixel]
              __hclk__ += 1
            end
          when 251
            __bg_pattern_lut__ = __bg_pattern_lut_fetched__
            # raise unless __bg_pattern_lut_fetched__ ==
            #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
            #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            if __scroll_addr_5_14__ & 0x7000 != 0x7000
              __scroll_addr_5_14__ += 0x1000
            else
              mask = __scroll_addr_5_14__ & 0x03e0
              if mask == 0x03a0
                __scroll_addr_5_14__ ^= 0x0800
                __scroll_addr_5_14__ &= 0x0c00
              elsif mask == 0x03e0
                __scroll_addr_5_14__ &= 0x0c00
              else
                __scroll_addr_5_14__ = (__scroll_addr_5_14__ & 0x0fe0) + 32
              end
            end

            __name_io_addr__ = (__scroll_addr_0_4__ | __scroll_addr_5_14__) & 0x0fff | 0x2000 # make cache consistent
            if __scroll_addr_0_4__ < 0x001f
              __scroll_addr_0_4__ += 1
              __name_io_addr__ += 1 # make cache consistent
            else
              __scroll_addr_0_4__ = 0
              __scroll_addr_5_14__ ^= 0x0400
              __name_io_addr__ ^= 0x041f # make cache consistent
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 255
            __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          when 256
            __io_addr__ = __name_io_addr__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __sp_latch__ = 0xff
            __hclk__ += 1
          when 257
            __scroll_addr_0_4__ = __scroll_latch__ & 0x001f
            __scroll_addr_5_14__ = (__scroll_addr_5_14__ & 0x7be0) | (__scroll_latch__ & 0x0400)
            __name_io_addr__ = (__scroll_addr_0_4__ | __scroll_addr_5_14__) & 0x0fff | 0x2000 # make cache consistent
            __sp_visible__ = false
            __sp_active__ = false
            __hclk__ += 1
          when 258, 266, 274, 282, 290, 298, 306, 314, 599, 607, 615, 623, 631, 639, 647, 655
            # Nestopia uses open_name here?
            __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 2
          when 260, 268, 276, 284, 292, 300, 308
            buffer_idx = (__hclk__ - 260) / 2
            __io_addr__ = buffer_idx >= __sp_buffered__ ? __pattern_end__ : (flip_v = __sp_buffer__[buffer_idx + 2][7]; tmp = (__scanline__ - __sp_buffer__[buffer_idx]) ^ (flip_v * 0xf); byte1 = __sp_buffer__[buffer_idx + 1]; addr = __sp_height__ == 16 ? ((byte1 & 0x01) << 12) | ((byte1 & 0xfe) << 4) | (tmp[3] * 0x10) : __sp_base__ | byte1 << 4; addr | (tmp & 7))
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          when 261, 269, 277, 285, 293, 301, 309, 317
            if (__hclk__ - 261) / 2 < __sp_buffered__
              __io_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]
            end
            __hclk__ += 1
          when 262, 270, 278, 286, 294, 302, 310, 318
            __io_addr__ = __io_addr__ | 8
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 1
          when 263, 271, 279, 287, 295, 303, 311, 319
            buffer_idx = (__hclk__ - 263) / 2
            if buffer_idx < __sp_buffered__
              pat0 = __io_pattern__
              pat1 = __chr_mem__[__io_addr__ & 0x1fff]
              if pat0 != 0 || pat1 != 0
                byte2 = __sp_buffer__[buffer_idx + 2]
                pos = SP_PIXEL_POSITIONS[byte2[6]] # OAM byte2 bit6: "Flip horizontally" flag
                pat = (pat0 >> 1 & 0x55) | (pat1 & 0xaa) | ((pat0 & 0x55) | (pat1 << 1 & 0xaa)) << 8
                x_base = __sp_buffer__[buffer_idx + 3]
                palette_base = 0x10 + ((byte2 & 3) << 2) # OAM byte2 bit0-1: Palette
                __sp_visible__ ||= __sp_map__.clear
                8.times do |dx|
                  x = x_base + dx
                  clr = (pat >> (pos[dx] * 2)) & 3
                  if __sp_map__[x] || clr == 0
                    next
                  end
                  __sp_map__[x] = sprite = __sp_map_buffer__[x]
                  # sprite[0]: behind flag, sprite[1]: zero hit flag, sprite[2]: color
                  sprite[0] = byte2[5] == 1 # OAM byte2 bit5: "Behind background" flag
                  sprite[1] = buffer_idx == 0 && __sp_zero_in_line__
                  sprite[2] = palette_base + clr
                end
                __sp_active__ = __sp_enabled__
              end
            end
            __hclk__ += 1
          when 264, 272, 280, 288, 296, 304, 312, 349, 357, 365, 373, 381, 389, 397, 405, 413, 421, 429, 437, 445, 453, 461, 469, 477, 485, 493, 501, 509, 517, 525, 533, 541, 549, 557, 565, 573, 581, 589, 597, 605, 613, 621, 629, 637, 653
            __io_addr__ = __name_io_addr__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 2
          when 316
            buffer_idx = (__hclk__ - 260) / 2
            __io_addr__ = buffer_idx >= __sp_buffered__ ? __pattern_end__ : (flip_v = __sp_buffer__[buffer_idx + 2][7]; tmp = (__scanline__ - __sp_buffer__[buffer_idx]) ^ (flip_v * 0xf); byte1 = __sp_buffer__[buffer_idx + 1]; addr = __sp_height__ == 16 ? ((byte1 & 0x01) << 12) | ((byte1 & 0xfe) << 4) | (tmp[3] * 0x10) : __sp_base__ | byte1 << 4; addr | (tmp & 7))
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            if __scanline__ == 238
              __regs_oam__ = 0
            end
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          when 320
            if 32 < __sp_buffered__
              buffer_idx = 32
              begin
                addr = open_sprite(buffer_idx)
                pat0 = __chr_mem__[addr]
                pat1 = __chr_mem__[addr | 8]
                if pat0 != 0 || pat1 != 0
                  byte2 = __sp_buffer__[buffer_idx + 2]
                  pos = SP_PIXEL_POSITIONS[byte2[6]] # OAM byte2 bit6: "Flip horizontally" flag
                  pat = (pat0 >> 1 & 0x55) | (pat1 & 0xaa) | ((pat0 & 0x55) | (pat1 << 1 & 0xaa)) << 8
                  x_base = __sp_buffer__[buffer_idx + 3]
                  palette_base = 0x10 + ((byte2 & 3) << 2) # OAM byte2 bit0-1: Palette
                  __sp_visible__ ||= __sp_map__.clear
                  8.times do |dx|
                    x = x_base + dx
                    clr = (pat >> (pos[dx] * 2)) & 3
                    if __sp_map__[x] || clr == 0
                      next
                    end
                    __sp_map__[x] = sprite = __sp_map_buffer__[x]
                    # sprite[0]: behind flag, sprite[1]: zero hit flag, sprite[2]: color
                    sprite[0] = byte2[5] == 1 # OAM byte2 bit5: "Behind background" flag
                    sprite[1] = buffer_idx == 0 && __sp_zero_in_line__
                    sprite[2] = palette_base + clr
                  end
                  __sp_active__ = __sp_enabled__
                end
                buffer_idx += 4
              end while buffer_idx != __sp_buffered__
            end
            __io_addr__ = __name_io_addr__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __sp_latch__ = __sp_ram__[0]
            __sp_buffered__ = 0
            __sp_zero_in_line__ = false
            __sp_index__ = 0
            __sp_phase__ = 0
            __hclk__ += 1
          when 321, 329
            __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]
            __hclk__ += 1
          when 322, 330
            __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 1
          when 323, 331
            __bg_pattern_lut__ = __bg_pattern_lut_fetched__
            # raise unless __bg_pattern_lut_fetched__ ==
            #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
            #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3
            if __scroll_addr_0_4__ < 0x001f
              __scroll_addr_0_4__ += 1
              __name_io_addr__ += 1 # make cache consistent
            else
              __scroll_addr_0_4__ = 0
              __scroll_addr_5_14__ ^= 0x0400
              __name_io_addr__ ^= 0x041f # make cache consistent
            end
            __hclk__ += 1
          when 324, 332
            __io_addr__ = __io_pattern__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 1
          when 325, 333
            __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]
            __hclk__ += 1
          when 326, 334
            __io_addr__ = __io_pattern__ | 8
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 1
          when 327, 335
            __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100
            __hclk__ += 1
          when 328
            __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
            __io_addr__ = __name_io_addr__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 1
          when 336
            __io_addr__ = __name_io_addr__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 1
          when 337
            __bg_enabled__ = __bg_show_edge__
            __sp_enabled__ = __sp_show_edge__
            __sp_active__ = __sp_enabled__ && __sp_visible__
            if __scanline__ == SCANLINE_HDUMMY && __odd_frame__
              __cpu__.next_frame_clock = RP2C02_HVSYNC_1
            end
            __hclk__ += 1
          when 338
            __io_addr__ = __name_io_addr__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __scanline__ += 1
            if __scanline__ != SCANLINE_VBLANK
              line = __scanline__ != 0 || !__odd_frame__ ? 341 : 340
              __hclk__ = 0
              __vclk__ += line
              __hclk_target__ = __hclk_target__ <= line ? 0 : __hclk_target__ - line
            else
              __hclk__ = HCLOCK_VBLANK_0
            end
          when 341
            __sp_overflow__ = __sp_zero_hit__ = __vblanking__ = __vblank__ = false
            __scanline__ = SCANLINE_HDUMMY
            __io_addr__ = __name_io_addr__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 2
          when 343, 351, 359, 367, 375, 383, 391, 399, 407, 415, 423, 431, 439, 447, 455, 463, 471, 479, 487, 495, 503, 511, 519, 527, 535, 543, 551, 559, 567, 575, 583, 591
            __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 2
          when 345, 353, 361, 369, 377, 385, 393, 401, 409, 417, 425, 433, 441, 449, 457, 465, 473, 481, 489, 497, 505, 513, 521, 529, 537, 545, 553, 561, 569, 577, 585, 593
            __io_addr__ = __bg_pattern_base__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 2
          when 347, 355, 363, 371, 379, 387, 395, 403, 411, 419, 427, 435, 443, 451, 459, 467, 475, 483, 491, 499, 507, 515, 523, 531, 539, 547, 555, 563, 571, 579, 587, 595, 603, 611, 619, 627, 635, 643, 651
            __io_addr__ = __io_addr__ | 8
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 2
          when 601, 609, 617, 625, 633, 641, 649, 657
            __io_addr__ = __pattern_end__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 2
          when 645
            __scroll_addr_0_4__  = __scroll_latch__ & 0x001f
            __scroll_addr_5_14__ = __scroll_latch__ & 0x7fe0
            __name_io_addr__ = (__scroll_addr_0_4__ | __scroll_addr_5_14__) & 0x0fff | 0x2000 # make cache consistent
            __io_addr__ = __name_io_addr__
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ += 2
          when 659
            __io_addr__ = __io_addr__ | 8
            a12_state = __io_addr__[12] == 1
            if !__a12_state__ && a12_state
              __a12_monitor__.a12_signaled((__vclk__ + __hclk__) * RP2C02_CC)
            end
            __a12_state__ = a12_state
            __hclk__ = 320
            __vclk__ += HCLOCK_DUMMY
            __hclk_target__ -= HCLOCK_DUMMY
          when 681
            __vblanking__ = true
            __hclk__ = HCLOCK_VBLANK_1
          when 682
            __vblank__ ||= __vblanking__
            __vblanking__ = false
            __sp_visible__ = false
            __sp_active__ = false
            __hclk__ = HCLOCK_VBLANK_2
          when 684
            __vblank__ ||= __vblanking__
            __vblanking__ = false
            __hclk__ = HCLOCK_DUMMY
            __hclk_target__ = FOREVER_CLOCK
            if __need_nmi__ && __vblank__
              __cpu__.do_nmi(__cpu__.next_frame_clock)
            end
            return
          when 685

            # wait for boot
            __vblank__ = true
            __hclk__ = HCLOCK_DUMMY
            __hclk_target__ = FOREVER_CLOCK
            return
          end
        end
      else
        while __hclk_target__ > __hclk__
          case __hclk__
          when 0, 8, 16, 24, 32, 40, 48, 56
            if __hclk__ + 8 <= __hclk_target__
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              # batch-version of render_pixel
              if __sp_active__
                if __bg_enabled__
                  pixel0 = __bg_pixels__[0]
                  if sprite = __sp_map__[__hclk__]
                    if pixel0 % 4 == 0
                      pixel0 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel0 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel1 = __bg_pixels__[1]
                  if sprite = __sp_map__[__hclk__ + 1]
                    if pixel1 % 4 == 0
                      pixel1 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel1 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel2 = __bg_pixels__[2]
                  if sprite = __sp_map__[__hclk__ + 2]
                    if pixel2 % 4 == 0
                      pixel2 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel2 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel3 = __bg_pixels__[3]
                  if sprite = __sp_map__[__hclk__ + 3]
                    if pixel3 % 4 == 0
                      pixel3 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel3 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel4 = __bg_pixels__[4]
                  if sprite = __sp_map__[__hclk__ + 4]
                    if pixel4 % 4 == 0
                      pixel4 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel4 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel5 = __bg_pixels__[5]
                  if sprite = __sp_map__[__hclk__ + 5]
                    if pixel5 % 4 == 0
                      pixel5 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel5 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel6 = __bg_pixels__[6]
                  if sprite = __sp_map__[__hclk__ + 6]
                    if pixel6 % 4 == 0
                      pixel6 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel6 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel7 = __bg_pixels__[7]
                  if sprite = __sp_map__[__hclk__ + 7]
                    if pixel7 % 4 == 0
                      pixel7 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel7 = sprite[2] unless sprite[0]
                    end
                  end
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                else
                  pixel0 = (sprite = __sp_map__[__hclk__ ]) ? sprite[2] : 0
                  pixel1 = (sprite = __sp_map__[__hclk__  + 1]) ? sprite[2] : 0
                  pixel2 = (sprite = __sp_map__[__hclk__  + 2]) ? sprite[2] : 0
                  pixel3 = (sprite = __sp_map__[__hclk__  + 3]) ? sprite[2] : 0
                  pixel4 = (sprite = __sp_map__[__hclk__  + 4]) ? sprite[2] : 0
                  pixel5 = (sprite = __sp_map__[__hclk__  + 5]) ? sprite[2] : 0
                  pixel6 = (sprite = __sp_map__[__hclk__  + 6]) ? sprite[2] : 0
                  pixel7 = (sprite = __sp_map__[__hclk__  + 7]) ? sprite[2] : 0
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                end
              else
                if __bg_enabled__ # this is the true hot-spot
                  __output_pixels__ << __output_color__[__bg_pixels__[0]] << __output_color__[__bg_pixels__[1]] << __output_color__[__bg_pixels__[2]] << __output_color__[__bg_pixels__[3]] << __output_color__[__bg_pixels__[4]] << __output_color__[__bg_pixels__[5]] << __output_color__[__bg_pixels__[6]] << __output_color__[__bg_pixels__[7]]
                else
                  clr = __output_color__[0]
                  __output_pixels__ << clr << clr << clr << clr << clr << clr << clr << clr
                end
              end
              __io_addr__ = __name_io_addr__
              __hclk__ += 1
              __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]
              __hclk__ += 1
              __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
              __hclk__ += 1
              __bg_pattern_lut__ = __bg_pattern_lut_fetched__
              # raise unless __bg_pattern_lut_fetched__ ==
              #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
              #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3
              if __scroll_addr_0_4__ < 0x001f
                __scroll_addr_0_4__ += 1
                __name_io_addr__ += 1 # make cache consistent
              else
                __scroll_addr_0_4__ = 0
                __scroll_addr_5_14__ ^= 0x0400
                __name_io_addr__ ^= 0x041f # make cache consistent
              end
              __hclk__ += 1
              __io_addr__ = __io_pattern__
              __hclk__ += 1
              __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]
              __hclk__ += 1
              __io_addr__ = __io_pattern__ | 8
              __hclk__ += 1
              __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100
              # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __bg_enabled__ = __bg_show__
              __sp_enabled__ = __sp_show__
              __sp_active__ = __sp_enabled__ && __sp_visible__
              # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __hclk__ += 1
            else
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              __io_addr__ = __name_io_addr__
              pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
              if __sp_active__ && (sprite = __sp_map__[__hclk__])
                if pixel % 4 == 0
                  pixel = sprite[2]
                else
                  if sprite[1] && __hclk__ != 255
                    __sp_zero_hit__ = true
                  end
                  unless sprite[0]
                    pixel = sprite[2]
                  end
                end
              end
              __output_pixels__ << __output_color__[pixel]
              __hclk__ += 1
            end
          when 1, 9, 17, 25, 33, 41, 49, 57
            __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 2, 10, 18, 26, 34, 42, 50, 58
            __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 3, 11, 19, 27, 35, 43, 51, 59
            __bg_pattern_lut__ = __bg_pattern_lut_fetched__
            # raise unless __bg_pattern_lut_fetched__ ==
            #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
            #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3
            if __scroll_addr_0_4__ < 0x001f
              __scroll_addr_0_4__ += 1
              __name_io_addr__ += 1 # make cache consistent
            else
              __scroll_addr_0_4__ = 0
              __scroll_addr_5_14__ ^= 0x0400
              __name_io_addr__ ^= 0x041f # make cache consistent
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 4, 12, 20, 28, 36, 44, 52, 60
            __io_addr__ = __io_pattern__
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 5, 13, 21, 29, 37, 45, 53, 61
            __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 6, 14, 22, 30, 38, 46, 54, 62
            __io_addr__ = __io_pattern__ | 8
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 7, 15, 23, 31, 39, 47, 55, 63
            __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __bg_enabled__ = __bg_show__
            __sp_enabled__ = __sp_show__
            __sp_active__ = __sp_enabled__ && __sp_visible__
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          when 64
            if __hclk__ + 8 <= __hclk_target__
              __sp_addr__ = __regs_oam__ & 0xf8 # SP_OFFSET_TO_0_1
              __sp_phase__ = nil
              __sp_latch__ = 0xff
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              # batch-version of render_pixel
              if __sp_active__
                if __bg_enabled__
                  pixel0 = __bg_pixels__[0]
                  if sprite = __sp_map__[__hclk__]
                    if pixel0 % 4 == 0
                      pixel0 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel0 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel1 = __bg_pixels__[1]
                  if sprite = __sp_map__[__hclk__ + 1]
                    if pixel1 % 4 == 0
                      pixel1 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel1 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel2 = __bg_pixels__[2]
                  if sprite = __sp_map__[__hclk__ + 2]
                    if pixel2 % 4 == 0
                      pixel2 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel2 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel3 = __bg_pixels__[3]
                  if sprite = __sp_map__[__hclk__ + 3]
                    if pixel3 % 4 == 0
                      pixel3 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel3 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel4 = __bg_pixels__[4]
                  if sprite = __sp_map__[__hclk__ + 4]
                    if pixel4 % 4 == 0
                      pixel4 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel4 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel5 = __bg_pixels__[5]
                  if sprite = __sp_map__[__hclk__ + 5]
                    if pixel5 % 4 == 0
                      pixel5 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel5 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel6 = __bg_pixels__[6]
                  if sprite = __sp_map__[__hclk__ + 6]
                    if pixel6 % 4 == 0
                      pixel6 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel6 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel7 = __bg_pixels__[7]
                  if sprite = __sp_map__[__hclk__ + 7]
                    if pixel7 % 4 == 0
                      pixel7 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel7 = sprite[2] unless sprite[0]
                    end
                  end
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                else
                  pixel0 = (sprite = __sp_map__[__hclk__ ]) ? sprite[2] : 0
                  pixel1 = (sprite = __sp_map__[__hclk__  + 1]) ? sprite[2] : 0
                  pixel2 = (sprite = __sp_map__[__hclk__  + 2]) ? sprite[2] : 0
                  pixel3 = (sprite = __sp_map__[__hclk__  + 3]) ? sprite[2] : 0
                  pixel4 = (sprite = __sp_map__[__hclk__  + 4]) ? sprite[2] : 0
                  pixel5 = (sprite = __sp_map__[__hclk__  + 5]) ? sprite[2] : 0
                  pixel6 = (sprite = __sp_map__[__hclk__  + 6]) ? sprite[2] : 0
                  pixel7 = (sprite = __sp_map__[__hclk__  + 7]) ? sprite[2] : 0
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                end
              else
                if __bg_enabled__ # this is the true hot-spot
                  __output_pixels__ << __output_color__[__bg_pixels__[0]] << __output_color__[__bg_pixels__[1]] << __output_color__[__bg_pixels__[2]] << __output_color__[__bg_pixels__[3]] << __output_color__[__bg_pixels__[4]] << __output_color__[__bg_pixels__[5]] << __output_color__[__bg_pixels__[6]] << __output_color__[__bg_pixels__[7]]
                else
                  clr = __output_color__[0]
                  __output_pixels__ << clr << clr << clr << clr << clr << clr << clr << clr
                end
              end
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              __hclk__ += 1
              __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
              __hclk__ += 1
              __bg_pattern_lut__ = __bg_pattern_lut_fetched__
              # raise unless __bg_pattern_lut_fetched__ ==
              #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
              #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              if __scroll_addr_0_4__ < 0x001f
                __scroll_addr_0_4__ += 1
                __name_io_addr__ += 1 # make cache consistent
              else
                __scroll_addr_0_4__ = 0
                __scroll_addr_5_14__ ^= 0x0400
                __name_io_addr__ ^= 0x041f # make cache consistent
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__
              __hclk__ += 1
              __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__ | 8
              __hclk__ += 1
              __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __bg_enabled__ = __bg_show__
              __sp_enabled__ = __sp_show__
              __sp_active__ = __sp_enabled__ && __sp_visible__
              # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __hclk__ += 1
            else
              __sp_addr__ = __regs_oam__ & 0xf8 # SP_OFFSET_TO_0_1
              __sp_phase__ = nil
              __sp_latch__ = 0xff
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
              if __sp_active__ && (sprite = __sp_map__[__hclk__])
                if pixel % 4 == 0
                  pixel = sprite[2]
                else
                  if sprite[1] && __hclk__ != 255
                    __sp_zero_hit__ = true
                  end
                  unless sprite[0]
                    pixel = sprite[2]
                  end
                end
              end
              __output_pixels__ << __output_color__[pixel]
              __hclk__ += 1
            end
          when 65, 73, 81, 89, 97, 105, 113, 121, 129, 137, 145, 153, 161, 169, 177, 185, 193, 201, 209, 217, 225, 233, 241, 249
            __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 66, 74, 82, 90, 98, 106, 114, 122, 130, 138, 146, 154, 162, 170, 178, 186, 194, 202, 210, 218, 226, 234, 242, 250
            __sp_latch__ = __sp_ram__[__sp_addr__]
            __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 67, 75, 83, 91, 99, 107, 115, 123, 131, 139, 147, 155, 163, 171, 179, 187, 195, 203, 211, 219, 227, 235, 243
            __bg_pattern_lut__ = __bg_pattern_lut_fetched__
            # raise unless __bg_pattern_lut_fetched__ ==
            #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
            #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            if __scroll_addr_0_4__ < 0x001f
              __scroll_addr_0_4__ += 1
              __name_io_addr__ += 1 # make cache consistent
            else
              __scroll_addr_0_4__ = 0
              __scroll_addr_5_14__ ^= 0x0400
              __name_io_addr__ ^= 0x041f # make cache consistent
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 68, 76, 84, 92, 100, 108, 116, 124, 132, 140, 148, 156, 164, 172, 180, 188, 196, 204, 212, 220, 228, 236, 244, 252
            __sp_latch__ = __sp_ram__[__sp_addr__]
            __io_addr__ = __io_pattern__
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 69, 77, 85, 93, 101, 109, 117, 125, 133, 141, 149, 157, 165, 173, 181, 189, 197, 205, 213, 221, 229, 237, 245, 253
            __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 70, 78, 86, 94, 102, 110, 118, 126, 134, 142, 150, 158, 166, 174, 182, 190, 198, 206, 214, 222, 230, 238, 246, 254
            __sp_latch__ = __sp_ram__[__sp_addr__]
            __io_addr__ = __io_pattern__ | 8
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 71, 79, 87, 95, 103, 111, 119, 127, 135, 143, 151, 159, 167, 175, 183, 191, 199, 207, 215, 223, 231, 239, 247
            __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __bg_enabled__ = __bg_show__
            __sp_enabled__ = __sp_show__
            __sp_active__ = __sp_enabled__ && __sp_visible__
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          when 72, 80, 88, 96, 104, 112, 120, 128, 136, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216, 224, 232, 240
            if __hclk__ + 8 <= __hclk_target__
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              # batch-version of render_pixel
              if __sp_active__
                if __bg_enabled__
                  pixel0 = __bg_pixels__[0]
                  if sprite = __sp_map__[__hclk__]
                    if pixel0 % 4 == 0
                      pixel0 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel0 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel1 = __bg_pixels__[1]
                  if sprite = __sp_map__[__hclk__ + 1]
                    if pixel1 % 4 == 0
                      pixel1 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel1 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel2 = __bg_pixels__[2]
                  if sprite = __sp_map__[__hclk__ + 2]
                    if pixel2 % 4 == 0
                      pixel2 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel2 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel3 = __bg_pixels__[3]
                  if sprite = __sp_map__[__hclk__ + 3]
                    if pixel3 % 4 == 0
                      pixel3 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel3 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel4 = __bg_pixels__[4]
                  if sprite = __sp_map__[__hclk__ + 4]
                    if pixel4 % 4 == 0
                      pixel4 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel4 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel5 = __bg_pixels__[5]
                  if sprite = __sp_map__[__hclk__ + 5]
                    if pixel5 % 4 == 0
                      pixel5 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel5 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel6 = __bg_pixels__[6]
                  if sprite = __sp_map__[__hclk__ + 6]
                    if pixel6 % 4 == 0
                      pixel6 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel6 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel7 = __bg_pixels__[7]
                  if sprite = __sp_map__[__hclk__ + 7]
                    if pixel7 % 4 == 0
                      pixel7 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel7 = sprite[2] unless sprite[0]
                    end
                  end
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                else
                  pixel0 = (sprite = __sp_map__[__hclk__ ]) ? sprite[2] : 0
                  pixel1 = (sprite = __sp_map__[__hclk__  + 1]) ? sprite[2] : 0
                  pixel2 = (sprite = __sp_map__[__hclk__  + 2]) ? sprite[2] : 0
                  pixel3 = (sprite = __sp_map__[__hclk__  + 3]) ? sprite[2] : 0
                  pixel4 = (sprite = __sp_map__[__hclk__  + 4]) ? sprite[2] : 0
                  pixel5 = (sprite = __sp_map__[__hclk__  + 5]) ? sprite[2] : 0
                  pixel6 = (sprite = __sp_map__[__hclk__  + 6]) ? sprite[2] : 0
                  pixel7 = (sprite = __sp_map__[__hclk__  + 7]) ? sprite[2] : 0
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                end
              else
                if __bg_enabled__ # this is the true hot-spot
                  __output_pixels__ << __output_color__[__bg_pixels__[0]] << __output_color__[__bg_pixels__[1]] << __output_color__[__bg_pixels__[2]] << __output_color__[__bg_pixels__[3]] << __output_color__[__bg_pixels__[4]] << __output_color__[__bg_pixels__[5]] << __output_color__[__bg_pixels__[6]] << __output_color__[__bg_pixels__[7]]
                else
                  clr = __output_color__[0]
                  __output_pixels__ << clr << clr << clr << clr << clr << clr << clr << clr
                end
              end
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              __hclk__ += 1
              __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
              __hclk__ += 1
              __bg_pattern_lut__ = __bg_pattern_lut_fetched__
              # raise unless __bg_pattern_lut_fetched__ ==
              #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
              #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              if __scroll_addr_0_4__ < 0x001f
                __scroll_addr_0_4__ += 1
                __name_io_addr__ += 1 # make cache consistent
              else
                __scroll_addr_0_4__ = 0
                __scroll_addr_5_14__ ^= 0x0400
                __name_io_addr__ ^= 0x041f # make cache consistent
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__
              __hclk__ += 1
              __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__ | 8
              __hclk__ += 1
              __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __bg_enabled__ = __bg_show__
              __sp_enabled__ = __sp_show__
              __sp_active__ = __sp_enabled__ && __sp_visible__
              # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __hclk__ += 1
            else
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
              if __sp_active__ && (sprite = __sp_map__[__hclk__])
                if pixel % 4 == 0
                  pixel = sprite[2]
                else
                  if sprite[1] && __hclk__ != 255
                    __sp_zero_hit__ = true
                  end
                  unless sprite[0]
                    pixel = sprite[2]
                  end
                end
              end
              __output_pixels__ << __output_color__[pixel]
              __hclk__ += 1
            end
          when 248
            if __hclk__ + 8 <= __hclk_target__
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              # batch-version of render_pixel
              if __sp_active__
                if __bg_enabled__
                  pixel0 = __bg_pixels__[0]
                  if sprite = __sp_map__[__hclk__]
                    if pixel0 % 4 == 0
                      pixel0 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel0 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel1 = __bg_pixels__[1]
                  if sprite = __sp_map__[__hclk__ + 1]
                    if pixel1 % 4 == 0
                      pixel1 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel1 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel2 = __bg_pixels__[2]
                  if sprite = __sp_map__[__hclk__ + 2]
                    if pixel2 % 4 == 0
                      pixel2 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel2 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel3 = __bg_pixels__[3]
                  if sprite = __sp_map__[__hclk__ + 3]
                    if pixel3 % 4 == 0
                      pixel3 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel3 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel4 = __bg_pixels__[4]
                  if sprite = __sp_map__[__hclk__ + 4]
                    if pixel4 % 4 == 0
                      pixel4 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel4 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel5 = __bg_pixels__[5]
                  if sprite = __sp_map__[__hclk__ + 5]
                    if pixel5 % 4 == 0
                      pixel5 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel5 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel6 = __bg_pixels__[6]
                  if sprite = __sp_map__[__hclk__ + 6]
                    if pixel6 % 4 == 0
                      pixel6 = sprite[2]
                    else
                      __sp_zero_hit__ = true if sprite[1]
                      pixel6 = sprite[2] unless sprite[0]
                    end
                  end
                  pixel7 = __bg_pixels__[7]
                  if sprite = __sp_map__[__hclk__ + 7]
                    if pixel7 % 4 == 0
                      pixel7 = sprite[2]
                    else
                      pixel7 = sprite[2] unless sprite[0]
                    end
                  end
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                else
                  pixel0 = (sprite = __sp_map__[__hclk__ ]) ? sprite[2] : 0
                  pixel1 = (sprite = __sp_map__[__hclk__  + 1]) ? sprite[2] : 0
                  pixel2 = (sprite = __sp_map__[__hclk__  + 2]) ? sprite[2] : 0
                  pixel3 = (sprite = __sp_map__[__hclk__  + 3]) ? sprite[2] : 0
                  pixel4 = (sprite = __sp_map__[__hclk__  + 4]) ? sprite[2] : 0
                  pixel5 = (sprite = __sp_map__[__hclk__  + 5]) ? sprite[2] : 0
                  pixel6 = (sprite = __sp_map__[__hclk__  + 6]) ? sprite[2] : 0
                  pixel7 = (sprite = __sp_map__[__hclk__  + 7]) ? sprite[2] : 0
                  __output_pixels__ << __output_color__[pixel0] << __output_color__[pixel1] << __output_color__[pixel2] << __output_color__[pixel3] << __output_color__[pixel4] << __output_color__[pixel5] << __output_color__[pixel6] << __output_color__[pixel7]
                end
              else
                if __bg_enabled__ # this is the true hot-spot
                  __output_pixels__ << __output_color__[__bg_pixels__[0]] << __output_color__[__bg_pixels__[1]] << __output_color__[__bg_pixels__[2]] << __output_color__[__bg_pixels__[3]] << __output_color__[__bg_pixels__[4]] << __output_color__[__bg_pixels__[5]] << __output_color__[__bg_pixels__[6]] << __output_color__[__bg_pixels__[7]]
                else
                  clr = __output_color__[0]
                  __output_pixels__ << clr << clr << clr << clr << clr << clr << clr << clr
                end
              end
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              __hclk__ += 1
              __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
              __hclk__ += 1
              __bg_pattern_lut__ = __bg_pattern_lut_fetched__
              # raise unless __bg_pattern_lut_fetched__ ==
              #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
              #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              if __scroll_addr_5_14__ & 0x7000 != 0x7000
                __scroll_addr_5_14__ += 0x1000
              else
                mask = __scroll_addr_5_14__ & 0x03e0
                if mask == 0x03a0
                  __scroll_addr_5_14__ ^= 0x0800
                  __scroll_addr_5_14__ &= 0x0c00
                elsif mask == 0x03e0
                  __scroll_addr_5_14__ &= 0x0c00
                else
                  __scroll_addr_5_14__ = (__scroll_addr_5_14__ & 0x0fe0) + 32
                end
              end

              __name_io_addr__ = (__scroll_addr_0_4__ | __scroll_addr_5_14__) & 0x0fff | 0x2000 # make cache consistent
              if __scroll_addr_0_4__ < 0x001f
                __scroll_addr_0_4__ += 1
                __name_io_addr__ += 1 # make cache consistent
              else
                __scroll_addr_0_4__ = 0
                __scroll_addr_5_14__ ^= 0x0400
                __name_io_addr__ ^= 0x041f # make cache consistent
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__
              __hclk__ += 1
              __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              __hclk__ += 1
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __io_pattern__ | 8
              __hclk__ += 1
              __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100

              # we first check phase 1 since it is the most-likely case
              if __sp_phase__ # nil represents phase 1
                # the second most-likely case is phase 9
                if __sp_phase__ == 9
                  __sp_addr__ = (__sp_addr__ + 4) & 0xff
                else
                  # other cases are relatively rare
                  case __sp_phase__
                  # when 1 then evaluate_sprites_odd_phase_1
                  # when 9 then evaluate_sprites_odd_phase_9
                  when 2
                    __sp_addr__ += 1
                    __sp_phase__ = 3
                    __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                  when 3
                    __sp_addr__ += 1
                    __sp_phase__ = 4
                    __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                  when 4
                    __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                    __sp_buffered__ += 4
                    if __sp_index__ != 64
                      __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                      if __sp_index__ != 2
                        __sp_addr__ += 1
                        __sp_zero_in_line__ ||= __sp_index__ == 1
                      else
                        __sp_addr__ = 8
                      end
                    else
                      __sp_addr__ = 0
                      __sp_phase__ = 9
                    end
                  when 5
                    if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                      __sp_phase__ = 6
                      __sp_addr__ = (__sp_addr__ + 1) & 0xff
                      __sp_overflow__ = true
                    else
                      __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                      if __sp_addr__ <= 5
                        __sp_phase__ = 9
                        __sp_addr__ &= 0xfc
                      end
                    end
                  when 6
                    __sp_phase__ = 7
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 7
                    __sp_phase__ = 8
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  when 8
                    __sp_phase__ = 9
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    if __sp_addr__ & 3 == 3
                      __sp_addr__ += 1
                    end
                    __sp_addr__ &= 0xfc
                  end
                end
              else
                __sp_index__ += 1
                if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                  __sp_addr__ += 1
                  __sp_phase__ = 2
                  __sp_buffer__[__sp_buffered__] = __sp_latch__
                elsif __sp_index__ == 64
                  __sp_addr__ = 0
                  __sp_phase__ = 9
                elsif __sp_index__ == 2
                  __sp_addr__ = 8
                else
                  __sp_addr__ += 4
                end
              end
              # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
              __hclk__ += 1
            else
              __bg_pixels__.rotate!(8)
              __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
              __sp_latch__ = __sp_ram__[__sp_addr__]
              __io_addr__ = __name_io_addr__
              pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
              if __sp_active__ && (sprite = __sp_map__[__hclk__])
                if pixel % 4 == 0
                  pixel = sprite[2]
                else
                  if sprite[1] && __hclk__ != 255
                    __sp_zero_hit__ = true
                  end
                  unless sprite[0]
                    pixel = sprite[2]
                  end
                end
              end
              __output_pixels__ << __output_color__[pixel]
              __hclk__ += 1
            end
          when 251
            __bg_pattern_lut__ = __bg_pattern_lut_fetched__
            # raise unless __bg_pattern_lut_fetched__ ==
            #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
            #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            if __scroll_addr_5_14__ & 0x7000 != 0x7000
              __scroll_addr_5_14__ += 0x1000
            else
              mask = __scroll_addr_5_14__ & 0x03e0
              if mask == 0x03a0
                __scroll_addr_5_14__ ^= 0x0800
                __scroll_addr_5_14__ &= 0x0c00
              elsif mask == 0x03e0
                __scroll_addr_5_14__ &= 0x0c00
              else
                __scroll_addr_5_14__ = (__scroll_addr_5_14__ & 0x0fe0) + 32
              end
            end

            __name_io_addr__ = (__scroll_addr_0_4__ | __scroll_addr_5_14__) & 0x0fff | 0x2000 # make cache consistent
            if __scroll_addr_0_4__ < 0x001f
              __scroll_addr_0_4__ += 1
              __name_io_addr__ += 1 # make cache consistent
            else
              __scroll_addr_0_4__ = 0
              __scroll_addr_5_14__ ^= 0x0400
              __name_io_addr__ ^= 0x041f # make cache consistent
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          when 255
            __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100

            # we first check phase 1 since it is the most-likely case
            if __sp_phase__ # nil represents phase 1
              # the second most-likely case is phase 9
              if __sp_phase__ == 9
                __sp_addr__ = (__sp_addr__ + 4) & 0xff
              else
                # other cases are relatively rare
                case __sp_phase__
                # when 1 then evaluate_sprites_odd_phase_1
                # when 9 then evaluate_sprites_odd_phase_9
                when 2
                  __sp_addr__ += 1
                  __sp_phase__ = 3
                  __sp_buffer__[__sp_buffered__ + 1] = __sp_latch__
                when 3
                  __sp_addr__ += 1
                  __sp_phase__ = 4
                  __sp_buffer__[__sp_buffered__ + 2] = __sp_latch__
                when 4
                  __sp_buffer__[__sp_buffered__ + 3] = __sp_latch__
                  __sp_buffered__ += 4
                  if __sp_index__ != 64
                    __sp_phase__ = __sp_buffered__ != __sp_limit__ ? nil : 5
                    if __sp_index__ != 2
                      __sp_addr__ += 1
                      __sp_zero_in_line__ ||= __sp_index__ == 1
                    else
                      __sp_addr__ = 8
                    end
                  else
                    __sp_addr__ = 0
                    __sp_phase__ = 9
                  end
                when 5
                  if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                    __sp_phase__ = 6
                    __sp_addr__ = (__sp_addr__ + 1) & 0xff
                    __sp_overflow__ = true
                  else
                    __sp_addr__ = ((__sp_addr__ + 4) & 0xfc) + ((__sp_addr__ + 1) & 3)
                    if __sp_addr__ <= 5
                      __sp_phase__ = 9
                      __sp_addr__ &= 0xfc
                    end
                  end
                when 6
                  __sp_phase__ = 7
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 7
                  __sp_phase__ = 8
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                when 8
                  __sp_phase__ = 9
                  __sp_addr__ = (__sp_addr__ + 1) & 0xff
                  if __sp_addr__ & 3 == 3
                    __sp_addr__ += 1
                  end
                  __sp_addr__ &= 0xfc
                end
              end
            else
              __sp_index__ += 1
              if __sp_latch__ <= __scanline__ && __scanline__ < __sp_latch__ + __sp_height__
                __sp_addr__ += 1
                __sp_phase__ = 2
                __sp_buffer__[__sp_buffered__] = __sp_latch__
              elsif __sp_index__ == 64
                __sp_addr__ = 0
                __sp_phase__ = 9
              elsif __sp_index__ == 2
                __sp_addr__ = 8
              else
                __sp_addr__ += 4
              end
            end
            pixel = __bg_enabled__ ? __bg_pixels__[__hclk__ % 8] : 0
            if __sp_active__ && (sprite = __sp_map__[__hclk__])
              if pixel % 4 == 0
                pixel = sprite[2]
              else
                if sprite[1] && __hclk__ != 255
                  __sp_zero_hit__ = true
                end
                unless sprite[0]
                  pixel = sprite[2]
                end
              end
            end
            __output_pixels__ << __output_color__[pixel]
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          when 256
            __io_addr__ = __name_io_addr__
            __sp_latch__ = 0xff
            __hclk__ += 1
          when 257
            __scroll_addr_0_4__ = __scroll_latch__ & 0x001f
            __scroll_addr_5_14__ = (__scroll_addr_5_14__ & 0x7be0) | (__scroll_latch__ & 0x0400)
            __name_io_addr__ = (__scroll_addr_0_4__ | __scroll_addr_5_14__) & 0x0fff | 0x2000 # make cache consistent
            __sp_visible__ = false
            __sp_active__ = false
            __hclk__ += 1
          when 258, 266, 274, 282, 290, 298, 306, 314, 599, 607, 615, 623, 631, 639, 647, 655
            # Nestopia uses open_name here?
            __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
            __hclk__ += 2
          when 260, 268, 276, 284, 292, 300, 308
            buffer_idx = (__hclk__ - 260) / 2
            __io_addr__ = buffer_idx >= __sp_buffered__ ? __pattern_end__ : (flip_v = __sp_buffer__[buffer_idx + 2][7]; tmp = (__scanline__ - __sp_buffer__[buffer_idx]) ^ (flip_v * 0xf); byte1 = __sp_buffer__[buffer_idx + 1]; addr = __sp_height__ == 16 ? ((byte1 & 0x01) << 12) | ((byte1 & 0xfe) << 4) | (tmp[3] * 0x10) : __sp_base__ | byte1 << 4; addr | (tmp & 7))
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          when 261, 269, 277, 285, 293, 301, 309, 317
            if (__hclk__ - 261) / 2 < __sp_buffered__
              __io_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]
            end
            __hclk__ += 1
          when 262, 270, 278, 286, 294, 302, 310, 318
            __io_addr__ = __io_addr__ | 8
            __hclk__ += 1
          when 263, 271, 279, 287, 295, 303, 311, 319
            buffer_idx = (__hclk__ - 263) / 2
            if buffer_idx < __sp_buffered__
              pat0 = __io_pattern__
              pat1 = __chr_mem__[__io_addr__ & 0x1fff]
              if pat0 != 0 || pat1 != 0
                byte2 = __sp_buffer__[buffer_idx + 2]
                pos = SP_PIXEL_POSITIONS[byte2[6]] # OAM byte2 bit6: "Flip horizontally" flag
                pat = (pat0 >> 1 & 0x55) | (pat1 & 0xaa) | ((pat0 & 0x55) | (pat1 << 1 & 0xaa)) << 8
                x_base = __sp_buffer__[buffer_idx + 3]
                palette_base = 0x10 + ((byte2 & 3) << 2) # OAM byte2 bit0-1: Palette
                __sp_visible__ ||= __sp_map__.clear
                8.times do |dx|
                  x = x_base + dx
                  clr = (pat >> (pos[dx] * 2)) & 3
                  if __sp_map__[x] || clr == 0
                    next
                  end
                  __sp_map__[x] = sprite = __sp_map_buffer__[x]
                  # sprite[0]: behind flag, sprite[1]: zero hit flag, sprite[2]: color
                  sprite[0] = byte2[5] == 1 # OAM byte2 bit5: "Behind background" flag
                  sprite[1] = buffer_idx == 0 && __sp_zero_in_line__
                  sprite[2] = palette_base + clr
                end
                __sp_active__ = __sp_enabled__
              end
            end
            __hclk__ += 1
          when 264, 272, 280, 288, 296, 304, 312, 349, 357, 365, 373, 381, 389, 397, 405, 413, 421, 429, 437, 445, 453, 461, 469, 477, 485, 493, 501, 509, 517, 525, 533, 541, 549, 557, 565, 573, 581, 589, 597, 605, 613, 621, 629, 637, 653
            __io_addr__ = __name_io_addr__
            __hclk__ += 2
          when 316
            buffer_idx = (__hclk__ - 260) / 2
            __io_addr__ = buffer_idx >= __sp_buffered__ ? __pattern_end__ : (flip_v = __sp_buffer__[buffer_idx + 2][7]; tmp = (__scanline__ - __sp_buffer__[buffer_idx]) ^ (flip_v * 0xf); byte1 = __sp_buffer__[buffer_idx + 1]; addr = __sp_height__ == 16 ? ((byte1 & 0x01) << 12) | ((byte1 & 0xfe) << 4) | (tmp[3] * 0x10) : __sp_base__ | byte1 << 4; addr | (tmp & 7))
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            if __scanline__ == 238
              __regs_oam__ = 0
            end
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          when 320
            if 32 < __sp_buffered__
              buffer_idx = 32
              begin
                addr = open_sprite(buffer_idx)
                pat0 = __chr_mem__[addr]
                pat1 = __chr_mem__[addr | 8]
                if pat0 != 0 || pat1 != 0
                  byte2 = __sp_buffer__[buffer_idx + 2]
                  pos = SP_PIXEL_POSITIONS[byte2[6]] # OAM byte2 bit6: "Flip horizontally" flag
                  pat = (pat0 >> 1 & 0x55) | (pat1 & 0xaa) | ((pat0 & 0x55) | (pat1 << 1 & 0xaa)) << 8
                  x_base = __sp_buffer__[buffer_idx + 3]
                  palette_base = 0x10 + ((byte2 & 3) << 2) # OAM byte2 bit0-1: Palette
                  __sp_visible__ ||= __sp_map__.clear
                  8.times do |dx|
                    x = x_base + dx
                    clr = (pat >> (pos[dx] * 2)) & 3
                    if __sp_map__[x] || clr == 0
                      next
                    end
                    __sp_map__[x] = sprite = __sp_map_buffer__[x]
                    # sprite[0]: behind flag, sprite[1]: zero hit flag, sprite[2]: color
                    sprite[0] = byte2[5] == 1 # OAM byte2 bit5: "Behind background" flag
                    sprite[1] = buffer_idx == 0 && __sp_zero_in_line__
                    sprite[2] = palette_base + clr
                  end
                  __sp_active__ = __sp_enabled__
                end
                buffer_idx += 4
              end while buffer_idx != __sp_buffered__
            end
            __io_addr__ = __name_io_addr__
            __sp_latch__ = __sp_ram__[0]
            __sp_buffered__ = 0
            __sp_zero_in_line__ = false
            __sp_index__ = 0
            __sp_phase__ = 0
            __hclk__ += 1
          when 321, 329
            __io_pattern__ = __name_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__ + __bg_pattern_base_15__]
            __hclk__ += 1
          when 322, 330
            __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
            __hclk__ += 1
          when 323, 331
            __bg_pattern_lut__ = __bg_pattern_lut_fetched__
            # raise unless __bg_pattern_lut_fetched__ ==
            #   __nmt_ref__[__io_addr__ >> 10 & 3][__io_addr__ & 0x03ff] >>
            #     ((__scroll_addr_0_4__ & 0x2) | (__scroll_addr_5_14__[6] * 0x4)) & 3
            if __scroll_addr_0_4__ < 0x001f
              __scroll_addr_0_4__ += 1
              __name_io_addr__ += 1 # make cache consistent
            else
              __scroll_addr_0_4__ = 0
              __scroll_addr_5_14__ ^= 0x0400
              __name_io_addr__ ^= 0x041f # make cache consistent
            end
            __hclk__ += 1
          when 324, 332
            __io_addr__ = __io_pattern__
            __hclk__ += 1
          when 325, 333
            __bg_pattern__ = __chr_mem__[__io_addr__ & 0x1fff]
            __hclk__ += 1
          when 326, 334
            __io_addr__ = __io_pattern__ | 8
            __hclk__ += 1
          when 327, 335
            __bg_pattern__ |= __chr_mem__[__io_addr__ & 0x1fff] * 0x100
            __hclk__ += 1
          when 328
            __bg_pixels__[__scroll_xfine__, 8] = __bg_pattern_lut__[__bg_pattern__]
            __io_addr__ = __name_io_addr__
            __hclk__ += 1
          when 336
            __io_addr__ = __name_io_addr__
            __hclk__ += 1
          when 337
            __bg_enabled__ = __bg_show_edge__
            __sp_enabled__ = __sp_show_edge__
            __sp_active__ = __sp_enabled__ && __sp_visible__
            if __scanline__ == SCANLINE_HDUMMY && __odd_frame__
              __cpu__.next_frame_clock = RP2C02_HVSYNC_1
            end
            __hclk__ += 1
          when 338
            __io_addr__ = __name_io_addr__
            __scanline__ += 1
            if __scanline__ != SCANLINE_VBLANK
              line = __scanline__ != 0 || !__odd_frame__ ? 341 : 340
              __hclk__ = 0
              __vclk__ += line
              __hclk_target__ = __hclk_target__ <= line ? 0 : __hclk_target__ - line
            else
              __hclk__ = HCLOCK_VBLANK_0
            end
          when 341
            __sp_overflow__ = __sp_zero_hit__ = __vblanking__ = __vblank__ = false
            __scanline__ = SCANLINE_HDUMMY
            __io_addr__ = __name_io_addr__
            __hclk__ += 2
          when 343, 351, 359, 367, 375, 383, 391, 399, 407, 415, 423, 431, 439, 447, 455, 463, 471, 479, 487, 495, 503, 511, 519, 527, 535, 543, 551, 559, 567, 575, 583, 591
            __io_addr__, __bg_pattern_lut_fetched__, = __attr_lut__[__scroll_addr_0_4__ + __scroll_addr_5_14__]
            __hclk__ += 2
          when 345, 353, 361, 369, 377, 385, 393, 401, 409, 417, 425, 433, 441, 449, 457, 465, 473, 481, 489, 497, 505, 513, 521, 529, 537, 545, 553, 561, 569, 577, 585, 593
            __io_addr__ = __bg_pattern_base__
            __hclk__ += 2
          when 347, 355, 363, 371, 379, 387, 395, 403, 411, 419, 427, 435, 443, 451, 459, 467, 475, 483, 491, 499, 507, 515, 523, 531, 539, 547, 555, 563, 571, 579, 587, 595, 603, 611, 619, 627, 635, 643, 651
            __io_addr__ = __io_addr__ | 8
            __hclk__ += 2
          when 601, 609, 617, 625, 633, 641, 649, 657
            __io_addr__ = __pattern_end__
            __hclk__ += 2
          when 645
            __scroll_addr_0_4__  = __scroll_latch__ & 0x001f
            __scroll_addr_5_14__ = __scroll_latch__ & 0x7fe0
            __name_io_addr__ = (__scroll_addr_0_4__ | __scroll_addr_5_14__) & 0x0fff | 0x2000 # make cache consistent
            __io_addr__ = __name_io_addr__
            __hclk__ += 2
          when 659
            __io_addr__ = __io_addr__ | 8
            __hclk__ = 320
            __vclk__ += HCLOCK_DUMMY
            __hclk_target__ -= HCLOCK_DUMMY
          when 681
            __vblanking__ = true
            __hclk__ = HCLOCK_VBLANK_1
          when 682
            __vblank__ ||= __vblanking__
            __vblanking__ = false
            __sp_visible__ = false
            __sp_active__ = false
            __hclk__ = HCLOCK_VBLANK_2
          when 684
            __vblank__ ||= __vblanking__
            __vblanking__ = false
            __hclk__ = HCLOCK_DUMMY
            __hclk_target__ = FOREVER_CLOCK
            if __need_nmi__ && __vblank__
              __cpu__.do_nmi(__cpu__.next_frame_clock)
            end
            return
          when 685

            # wait for boot
            __vblank__ = true
            __hclk__ = HCLOCK_DUMMY
            __hclk_target__ = FOREVER_CLOCK
            return
          end
        end
      end
    else
      while __hclk_target__ > __hclk__
        case __hclk__
        when 0, 8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 128, 136, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216, 224, 232, 240, 248
          if __hclk__ + 8 <= __hclk_target__
            __bg_pixels__[__hclk__ % 8] = 0
            __output_pixels__ << __output_color__[__scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0]
            __hclk__ += 1
            __bg_pixels__[__hclk__ % 8] = 0
            __output_pixels__ << __output_color__[__scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0]
            __hclk__ += 1
            __bg_pixels__[__hclk__ % 8] = 0
            __output_pixels__ << __output_color__[__scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0]
            __hclk__ += 1
            __bg_pixels__[__hclk__ % 8] = 0
            __output_pixels__ << __output_color__[__scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0]
            __hclk__ += 1
            __bg_pixels__[__hclk__ % 8] = 0
            __output_pixels__ << __output_color__[__scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0]
            __hclk__ += 1
            __bg_pixels__[__hclk__ % 8] = 0
            __output_pixels__ << __output_color__[__scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0]
            __hclk__ += 1
            __bg_pixels__[__hclk__ % 8] = 0
            __output_pixels__ << __output_color__[__scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0]
            __hclk__ += 1
            __bg_pixels__[__hclk__ % 8] = 0
            __output_pixels__ << __output_color__[__scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0]
            # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
            __hclk__ += 1
          else
            pixel = __scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0
            __bg_pixels__[__hclk__ % 8] = 0
            __output_pixels__ << __output_color__[pixel]
            __hclk__ += 1
          end
        when 1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14, 17, 18, 19, 20, 21, 22, 25, 26, 27, 28, 29, 30, 33, 34, 35, 36, 37, 38, 41, 42, 43, 44, 45, 46, 49, 50, 51, 52, 53, 54, 57, 58, 59, 60, 61, 62, 65, 66, 67, 68, 69, 70, 73, 74, 75, 76, 77, 78, 81, 82, 83, 84, 85, 86, 89, 90, 91, 92, 93, 94, 97, 98, 99, 100, 101, 102, 105, 106, 107, 108, 109, 110, 113, 114, 115, 116, 117, 118, 121, 122, 123, 124, 125, 126, 129, 130, 131, 132, 133, 134, 137, 138, 139, 140, 141, 142, 145, 146, 147, 148, 149, 150, 153, 154, 155, 156, 157, 158, 161, 162, 163, 164, 165, 166, 169, 170, 171, 172, 173, 174, 177, 178, 179, 180, 181, 182, 185, 186, 187, 188, 189, 190, 193, 194, 195, 196, 197, 198, 201, 202, 203, 204, 205, 206, 209, 210, 211, 212, 213, 214, 217, 218, 219, 220, 221, 222, 225, 226, 227, 228, 229, 230, 233, 234, 235, 236, 237, 238, 241, 242, 243, 244, 245, 246, 249, 250, 251, 252, 253, 254
          pixel = __scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0
          __bg_pixels__[__hclk__ % 8] = 0
          __output_pixels__ << __output_color__[pixel]
          __hclk__ += 1
        when 7, 15, 23, 31, 39, 47, 55, 63, 71, 79, 87, 95, 103, 111, 119, 127, 135, 143, 151, 159, 167, 175, 183, 191, 199, 207, 215, 223, 231, 239, 247, 255
          pixel = __scroll_addr_5_14__ & 0x3f00 == 0x3f00 ? __scroll_addr_0_4__ : 0
          __bg_pixels__[__hclk__ % 8] = 0
          __output_pixels__ << __output_color__[pixel]
          # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
          # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
          __hclk__ += 1
        when 256, 260, 261, 262, 263, 268, 269, 270, 271, 276, 277, 278, 279, 284, 285, 286, 287, 292, 293, 294, 295, 300, 301, 302, 303, 308, 309, 310, 311, 316, 317, 318, 319, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333, 334, 335, 336, 337
          __hclk__ += 1
        when 257
          __sp_visible__ = false
          __sp_active__ = false
          __hclk__ += 1
        when 258, 266, 274, 282, 290, 298, 306, 314, 599, 607, 615, 623, 631, 639, 647, 655
          # Nestopia uses open_name here?
          __hclk__ += 2
        when 264, 272, 280, 288, 296, 304, 312, 343, 345, 347, 349, 351, 353, 355, 357, 359, 361, 363, 365, 367, 369, 371, 373, 375, 377, 379, 381, 383, 385, 387, 389, 391, 393, 395, 397, 399, 401, 403, 405, 407, 409, 411, 413, 415, 417, 419, 421, 423, 425, 427, 429, 431, 433, 435, 437, 439, 441, 443, 445, 447, 449, 451, 453, 455, 457, 459, 461, 463, 465, 467, 469, 471, 473, 475, 477, 479, 481, 483, 485, 487, 489, 491, 493, 495, 497, 499, 501, 503, 505, 507, 509, 511, 513, 515, 517, 519, 521, 523, 525, 527, 529, 531, 533, 535, 537, 539, 541, 543, 545, 547, 549, 551, 553, 555, 557, 559, 561, 563, 565, 567, 569, 571, 573, 575, 577, 579, 581, 583, 585, 587, 589, 591, 593, 595, 597, 601, 603, 605, 609, 611, 613, 617, 619, 621, 625, 627, 629, 633, 635, 637, 641, 643, 645, 649, 651, 653, 657
          __hclk__ += 2
        when 320
          __sp_buffered__ = 0
          __sp_zero_in_line__ = false
          __sp_index__ = 0
          __sp_phase__ = 0
          __hclk__ += 1
        when 338
          __scanline__ += 1
          if __scanline__ != SCANLINE_VBLANK
            __bg_enabled__ = __bg_show_edge__
            __sp_enabled__ = __sp_show_edge__
            __sp_active__ = __sp_enabled__ && __sp_visible__
            line = 341
            __hclk__ = 0
            __vclk__ += line
            __hclk_target__ = __hclk_target__ <= line ? 0 : __hclk_target__ - line
          else
            __hclk__ = HCLOCK_VBLANK_0
          end
        when 341
          __sp_overflow__ = __sp_zero_hit__ = __vblanking__ = __vblank__ = false
          __scanline__ = SCANLINE_HDUMMY
          __hclk__ += 2
        when 659
          __hclk__ = 320
          __vclk__ += HCLOCK_DUMMY
          __hclk_target__ -= HCLOCK_DUMMY
        when 681
          __vblanking__ = true
          __hclk__ = HCLOCK_VBLANK_1
        when 682
          __vblank__ ||= __vblanking__
          __vblanking__ = false
          __sp_visible__ = false
          __sp_active__ = false
          __hclk__ = HCLOCK_VBLANK_2
        when 684
          __vblank__ ||= __vblanking__
          __vblanking__ = false
          __hclk__ = HCLOCK_DUMMY
          __hclk_target__ = FOREVER_CLOCK
          if __need_nmi__ && __vblank__
            __cpu__.do_nmi(__cpu__.next_frame_clock)
          end
          return
        when 685

          # wait for boot
          __vblank__ = true
          __hclk__ = HCLOCK_DUMMY
          __hclk_target__ = FOREVER_CLOCK
          return
        end
      end
    end
    __hclk_target__ = (__vclk__ + __hclk__) * RP2C02_CC
  ensure
    @a12_monitor = __a12_monitor__
    @a12_state = __a12_state__
    @any_show = __any_show__
    @attr_lut = __attr_lut__
    @bg_enabled = __bg_enabled__
    @bg_pattern = __bg_pattern__
    @bg_pattern_base = __bg_pattern_base__
    @bg_pattern_base_15 = __bg_pattern_base_15__
    @bg_pattern_lut = __bg_pattern_lut__
    @bg_pattern_lut_fetched = __bg_pattern_lut_fetched__
    @bg_pixels = __bg_pixels__
    @bg_show = __bg_show__
    @bg_show_edge = __bg_show_edge__
    @chr_mem = __chr_mem__
    @cpu = __cpu__
    @hclk = __hclk__
    @hclk_target = __hclk_target__
    @io_addr = __io_addr__
    @io_pattern = __io_pattern__
    @name_io_addr = __name_io_addr__
    @name_lut = __name_lut__
    @need_nmi = __need_nmi__
    @nmt_ref = __nmt_ref__
    @odd_frame = __odd_frame__
    @output_color = __output_color__
    @output_pixels = __output_pixels__
    @pattern_end = __pattern_end__
    @regs_oam = __regs_oam__
    @scanline = __scanline__
    @scroll_addr_0_4 = __scroll_addr_0_4__
    @scroll_addr_5_14 = __scroll_addr_5_14__
    @scroll_latch = __scroll_latch__
    @scroll_xfine = __scroll_xfine__
    @sp_active = __sp_active__
    @sp_addr = __sp_addr__
    @sp_base = __sp_base__
    @sp_buffer = __sp_buffer__
    @sp_buffered = __sp_buffered__
    @sp_enabled = __sp_enabled__
    @sp_height = __sp_height__
    @sp_index = __sp_index__
    @sp_latch = __sp_latch__
    @sp_limit = __sp_limit__
    @sp_map = __sp_map__
    @sp_map_buffer = __sp_map_buffer__
    @sp_overflow = __sp_overflow__
    @sp_phase = __sp_phase__
    @sp_ram = __sp_ram__
    @sp_show = __sp_show__
    @sp_show_edge = __sp_show_edge__
    @sp_visible = __sp_visible__
    @sp_zero_hit = __sp_zero_hit__
    @sp_zero_in_line = __sp_zero_in_line__
    @vblank = __vblank__
    @vblanking = __vblanking__
    @vclk = __vclk__
  end
end
[VBLANK] 128
[VBLANK] 128
[VBLANK] 143
fps: 156.49724397342902
checksum: 59662
