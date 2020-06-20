def self.run
  @name_io_addr = (@scroll_addr_0_4 | @scroll_addr_5_14) & 0x0fff | 0x2000
  @bg_pattern_lut_fetched = TILE_LUT[
    @nmt_ref[@io_addr >> 10 & 3][@io_addr & 0x03ff] >> ((@scroll_addr_0_4 & 0x2) | (@scroll_addr_5_14[6] * 0x4)) & 3
  ]
  while @hclk_target > @hclk
    case @hclk
    when 0, 8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 128, 136, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216, 224, 232, 240, 248
      if @any_show
        if @hclk == 64
          @sp_addr = @regs_oam & 0xf8 # SP_OFFSET_TO_0_1
          @sp_phase = nil
          @sp_latch = 0xff
        end
        load_tiles
        batch_render_eight_pixels
        if @hclk >= 64
          evaluate_sprites_even
        end
        open_name
      end
      render_pixel
      @hclk += 1
    when 1, 9, 17, 25, 33, 41, 49, 57, 65, 73, 81, 89, 97, 105, 113, 121, 129, 137, 145, 153, 161, 169, 177, 185, 193, 201, 209, 217, 225, 233, 241, 249
      if @any_show
        fetch_name
        if @hclk >= 64
          evaluate_sprites_odd
        end
      end
      render_pixel
      @hclk += 1
    when 2, 10, 18, 26, 34, 42, 50, 58, 66, 74, 82, 90, 98, 106, 114, 122, 130, 138, 146, 154, 162, 170, 178, 186, 194, 202, 210, 218, 226, 234, 242, 250
      if @any_show
        if @hclk >= 64
          evaluate_sprites_even
        end
        open_attr
      end
      render_pixel
      @hclk += 1
    when 3, 11, 19, 27, 35, 43, 51, 59, 67, 75, 83, 91, 99, 107, 115, 123, 131, 139, 147, 155, 163, 171, 179, 187, 195, 203, 211, 219, 227, 235, 243, 251
      if @any_show
        fetch_attr
        if @hclk >= 64
          evaluate_sprites_odd
        end
        if @hclk == 251
          scroll_clock_y
        end
        scroll_clock_x
      end
      render_pixel
      @hclk += 1
    when 4, 12, 20, 28, 36, 44, 52, 60, 68, 76, 84, 92, 100, 108, 116, 124, 132, 140, 148, 156, 164, 172, 180, 188, 196, 204, 212, 220, 228, 236, 244, 252
      if @any_show
        if @hclk >= 64
          evaluate_sprites_even
        end
        open_pattern(@io_pattern)
      end
      render_pixel
      @hclk += 1
    when 5, 13, 21, 29, 37, 45, 53, 61, 69, 77, 85, 93, 101, 109, 117, 125, 133, 141, 149, 157, 165, 173, 181, 189, 197, 205, 213, 221, 229, 237, 245, 253
      if @any_show
        fetch_bg_pattern_0
        if @hclk >= 64
          evaluate_sprites_odd
        end
      end
      render_pixel
      @hclk += 1
    when 6, 14, 22, 30, 38, 46, 54, 62, 70, 78, 86, 94, 102, 110, 118, 126, 134, 142, 150, 158, 166, 174, 182, 190, 198, 206, 214, 222, 230, 238, 246, 254
      if @any_show
        if @hclk >= 64
          evaluate_sprites_even
        end
        open_pattern(@io_pattern | 8)
      end
      render_pixel
      @hclk += 1
    when 7, 15, 23, 31, 39, 47, 55, 63, 71, 79, 87, 95, 103, 111, 119, 127, 135, 143, 151, 159, 167, 175, 183, 191, 199, 207, 215, 223, 231, 239, 247, 255
      if @any_show
        fetch_bg_pattern_1
        if @hclk >= 64
          evaluate_sprites_odd
        end
      end
      render_pixel
      # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
      if @any_show
        if @hclk != 255
          update_enabled_flags
        end
      end
      # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
      @hclk += 1
    when 256
      open_name
      if @any_show
        @sp_latch = 0xff
      end
      @hclk += 1
    when 257
      scroll_reset_x
      @sp_visible = false
      @sp_active = false
      @hclk += 1
    when 258, 266, 274, 282, 290, 298, 306, 314, 599, 607, 615, 623, 631, 639, 647, 655
      # Nestopia uses open_name here?
      open_attr
      @hclk += 2
    when 260, 268, 276, 284, 292, 300, 308, 316
      if @any_show
        buffer_idx = (@hclk - 260) / 2
        open_pattern(buffer_idx >= @sp_buffered ? @pattern_end : open_sprite(buffer_idx))
        # rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
        if @hclk == 316
          if @scanline == 238
            @regs_oam = 0
          end
        end
        # rubocop:enable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
      end
      @hclk += 1
    when 261, 269, 277, 285, 293, 301, 309, 317
      if @any_show
        if (@hclk - 261) / 2 < @sp_buffered
          @io_pattern = @chr_mem[@io_addr & 0x1fff]
        end
      end
      @hclk += 1
    when 262, 270, 278, 286, 294, 302, 310, 318
      open_pattern(@io_addr | 8)
      @hclk += 1
    when 263, 271, 279, 287, 295, 303, 311, 319
      if @any_show
        buffer_idx = (@hclk - 263) / 2
        if buffer_idx < @sp_buffered
          pat0 = @io_pattern
          pat1 = @chr_mem[@io_addr & 0x1fff]
          if pat0 != 0 || pat1 != 0
            load_sprite(pat0, pat1, buffer_idx)
          end
        end
      end
      @hclk += 1
    when 264, 272, 280, 288, 296, 304, 312
      open_name
      @hclk += 2
    when 320
      load_extended_sprites
      open_name
      if @any_show
        @sp_latch = @sp_ram[0]
      end
      @sp_buffered = 0
      @sp_zero_in_line = false
      @sp_index = 0
      @sp_phase = 0
      @hclk += 1
    when 321, 329
      fetch_name
      @hclk += 1
    when 322, 330
      open_attr
      @hclk += 1
    when 323, 331
      fetch_attr
      scroll_clock_x
      @hclk += 1
    when 324, 332
      open_pattern(@io_pattern)
      @hclk += 1
    when 325, 333
      fetch_bg_pattern_0
      @hclk += 1
    when 326, 334
      open_pattern(@io_pattern | 8)
      @hclk += 1
    when 327, 335
      fetch_bg_pattern_1
      @hclk += 1
    when 328
      preload_tiles
      open_name
      @hclk += 1
    when 336
      open_name
      @hclk += 1
    when 337
      if @any_show
        update_enabled_flags_edge
        if @scanline == SCANLINE_HDUMMY && @odd_frame
          @cpu.next_frame_clock = RP2C02_HVSYNC_1
        end
      end
      @hclk += 1
    when 338
      open_name
      @scanline += 1
      if @scanline != SCANLINE_VBLANK
        if @any_show
          line = @scanline != 0 || !@odd_frame ? 341 : 340
        else
          update_enabled_flags_edge
          line = 341
        end
        @hclk = 0
        @vclk += line
        @hclk_target = @hclk_target <= line ? 0 : @hclk_target - line
      else
        @hclk = HCLOCK_VBLANK_0
      end
    when 341, 349, 357, 365, 373, 381, 389, 397, 405, 413, 421, 429, 437, 445, 453, 461, 469, 477, 485, 493, 501, 509, 517, 525, 533, 541, 549, 557, 565, 573, 581, 589
      if @hclk == 341
        @sp_overflow = @sp_zero_hit = @vblanking = @vblank = false
        @scanline = SCANLINE_HDUMMY
      end
      open_name
      @hclk += 2
    when 343, 351, 359, 367, 375, 383, 391, 399, 407, 415, 423, 431, 439, 447, 455, 463, 471, 479, 487, 495, 503, 511, 519, 527, 535, 543, 551, 559, 567, 575, 583, 591
      open_attr
      @hclk += 2
    when 345, 353, 361, 369, 377, 385, 393, 401, 409, 417, 425, 433, 441, 449, 457, 465, 473, 481, 489, 497, 505, 513, 521, 529, 537, 545, 553, 561, 569, 577, 585, 593
      open_pattern(@bg_pattern_base)
      @hclk += 2
    when 347, 355, 363, 371, 379, 387, 395, 403, 411, 419, 427, 435, 443, 451, 459, 467, 475, 483, 491, 499, 507, 515, 523, 531, 539, 547, 555, 563, 571, 579, 587, 595
      open_pattern(@io_addr | 8)
      @hclk += 2
    when 597, 605, 613, 621, 629, 637, 645, 653
      if @any_show
        if @hclk == 645
          @scroll_addr_0_4  = @scroll_latch & 0x001f
          @scroll_addr_5_14 = @scroll_latch & 0x7fe0
          @name_io_addr = (@scroll_addr_0_4 | @scroll_addr_5_14) & 0x0fff | 0x2000 # make cache consistent
        end
      end
      open_name
      @hclk += 2
    when 601, 609, 617, 625, 633, 641, 649, 657
      open_pattern(@pattern_end)
      @hclk += 2
    when 603, 611, 619, 627, 635, 643, 651, 659
      open_pattern(@io_addr | 8)
      if @hclk == 659
        @hclk = 320
        @vclk += HCLOCK_DUMMY
        @hclk_target -= HCLOCK_DUMMY
      else
        @hclk += 2
      end
    when 681
      vblank_0
    when 682
      vblank_1
    when 684
      vblank_2
      return
    when 685

      # wait for boot
      boot
      return
    end
  end
  @hclk_target = (@vclk + @hclk) * RP2C02_CC
end
