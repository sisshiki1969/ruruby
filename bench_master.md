# ruruby benchmark results

## environment

Ruby version: 3.0.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 20.04.1 LTS

## execution time

|     benchmark      |      ruby      |     ruruby     |  rate  |
| :----------------: | :------------: | :------------: | :----: |
|   simple_call.rb   | 0.19 ± 0.01 s  | 0.44 ± 0.01 s  | x 2.35 |
|  accessor_get.rb   | 0.58 ± 0.00 s  | 0.97 ± 0.00 s  | x 1.67 |
|  accessor_set.rb   | 0.42 ± 0.00 s  | 1.17 ± 0.01 s  | x 2.80 |
|    ivar_get.rb     | 1.76 ± 0.03 s  | 1.34 ± 0.01 s  | x 0.76 |
|    ivar_set.rb     | 1.16 ± 0.02 s  | 2.23 ± 0.01 s  | x 1.93 |
|   loop_times.rb    | 0.77 ± 0.04 s  | 0.44 ± 0.01 s  | x 0.58 |
|    loop_for.rb     | 0.82 ± 0.01 s  | 0.50 ± 0.01 s  | x 0.61 |
| loop_whileloop.rb  | 0.41 ± 0.00 s  | 0.49 ± 0.01 s  | x 1.20 |
| so_concatenate.rb  | 0.71 ± 0.00 s  | 0.64 ± 0.01 s  | x 0.91 |
| string_scan_str.rb | 1.12 ± 0.01 s  | 1.11 ± 0.01 s  | x 0.99 |
| string_scan_re.rb  | 1.59 ± 0.01 s  | 1.07 ± 0.01 s  | x 0.67 |
| fiber_allocate.rb  | 1.53 ± 0.04 s  | 0.70 ± 0.01 s  | x 0.46 |
|  fiber_switch.rb   | 0.78 ± 0.01 s  | 1.23 ± 0.01 s  | x 1.58 |
|  so_mandelbrot.rb  | 1.82 ± 0.02 s  | 2.32 ± 0.06 s  | x 1.28 |
| app_mandelbrot.rb  | 1.62 ± 0.38 s  | 1.21 ± 0.13 s  | x 0.75 |
|    app_fibo.rb     | 0.54 ± 0.02 s  | 1.16 ± 0.02 s  | x 2.13 |
|   app_aobench.rb   | 10.18 ± 0.18 s | 17.88 ± 0.19 s | x 1.76 |
|    so_nbody.rb     | 1.08 ± 0.02 s  | 2.34 ± 0.06 s  | x 2.17 |
|     collatz.rb     | 6.30 ± 0.04 s  | 7.54 ± 0.06 s  | x 1.20 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 40.07 ± 2.07 fps  | 16.49 ± 0.17 fps | x 2.43 |
| optcarrot --opt | 131.59 ± 2.84 fps | 61.66 ± 7.34 fps | x 2.13 |

## memory consumption

|     benchmark      | ruby  | ruruby  |  rate   |
| :----------------: | :---: | :-----: | :-----: |
|   simple_call.rb   | 22.0M |  5.1M   | x 0.23  |
|  accessor_get.rb   | 22.0M |  5.0M   | x 0.23  |
|  accessor_set.rb   | 22.1M |  5.0M   | x 0.23  |
|    ivar_get.rb     | 22.0M |  5.0M   | x 0.23  |
|    ivar_set.rb     | 22.0M |  5.0M   | x 0.23  |
|   loop_times.rb    | 22.0M |  5.0M   | x 0.23  |
|    loop_for.rb     | 22.1M |  5.0M   | x 0.23  |
| loop_whileloop.rb  | 22.0M |  5.0M   | x 0.23  |
| so_concatenate.rb  | 71.9M |  63.6M  | x 0.88  |
| string_scan_str.rb | 26.6M |  6.6M   | x 0.25  |
| string_scan_re.rb  | 26.9M |  6.7M   | x 0.25  |
| fiber_allocate.rb  | 45.9M | 211.0M  | x 4.60  |
|  fiber_switch.rb   | 22.0M |  5.0M   | x 0.23  |
|  so_mandelbrot.rb  | 22.2M |  5.9M   | x 0.26  |
| app_mandelbrot.rb  | 22.1M |  5.8M   | x 0.26  |
|    app_fibo.rb     | 22.1M |  5.0M   | x 0.23  |
|   app_aobench.rb   | 22.7M |  8.1M   | x 0.36  |
|    so_nbody.rb     | 22.1M |  5.4M   | x 0.24  |
|     collatz.rb     | 22.1M |  5.1M   | x 0.23  |
|     optcarrot      | 78.3M |  64.6M  | x 0.83  |
|  optcarrot --opt   | 88.8M | 1051.8M | x 11.85 |
