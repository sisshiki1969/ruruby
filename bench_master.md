# ruruby benchmark results

## environment

Ruby version: 3.0.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 20.04.1 LTS

## execution time

|     benchmark      |     ruby      |     ruruby     |  rate  |
| :----------------: | :-----------: | :------------: | :----: |
|   simple_call.rb   | 0.19 ± 0.01 s | 0.37 ± 0.00 s  | x 1.98 |
|  accessor_get.rb   | 0.55 ± 0.01 s | 0.85 ± 0.02 s  | x 1.56 |
|  accessor_set.rb   | 0.40 ± 0.01 s | 1.04 ± 0.02 s  | x 2.62 |
|    ivar_get.rb     | 0.84 ± 0.01 s | 0.60 ± 0.01 s  | x 0.71 |
|    ivar_set.rb     | 0.56 ± 0.01 s | 1.02 ± 0.01 s  | x 1.80 |
|   loop_times.rb    | 0.72 ± 0.02 s | 0.35 ± 0.00 s  | x 0.49 |
|    loop_for.rb     | 0.80 ± 0.03 s | 0.41 ± 0.01 s  | x 0.51 |
| loop_whileloop.rb  | 0.38 ± 0.00 s | 0.41 ± 0.01 s  | x 1.06 |
| so_concatenate.rb  | 0.67 ± 0.01 s | 0.54 ± 0.01 s  | x 0.81 |
| string_scan_str.rb | 1.05 ± 0.00 s | 0.95 ± 0.03 s  | x 0.91 |
| string_scan_re.rb  | 1.58 ± 0.05 s | 0.96 ± 0.02 s  | x 0.61 |
| fiber_allocate.rb  | 1.29 ± 0.05 s | 0.57 ± 0.02 s  | x 0.44 |
|  fiber_switch.rb   | 0.74 ± 0.01 s | 1.10 ± 0.01 s  | x 1.47 |
|  so_mandelbrot.rb  | 1.66 ± 0.02 s | 2.02 ± 0.03 s  | x 1.22 |
| app_mandelbrot.rb  | 1.29 ± 0.03 s | 0.99 ± 0.01 s  | x 0.77 |
|    app_fibo.rb     | 0.49 ± 0.01 s | 1.00 ± 0.01 s  | x 2.03 |
|   app_aobench.rb   | 9.33 ± 0.18 s | 16.70 ± 0.30 s | x 1.79 |
|    so_nbody.rb     | 0.98 ± 0.01 s | 2.09 ± 0.06 s  | x 2.12 |
|     collatz.rb     | 5.96 ± 0.05 s | 6.89 ± 0.02 s  | x 1.16 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 42.84 ± 1.46 fps  | 18.84 ± 0.46 fps | x 2.27 |
| optcarrot --opt | 137.67 ± 5.62 fps | 69.49 ± 2.68 fps | x 1.98 |

## memory consumption

|     benchmark      | ruby  | ruruby |  rate  |
| :----------------: | :---: | :----: | :----: |
|   simple_call.rb   | 22.1M |  5.1M  | x 0.23 |
|  accessor_get.rb   | 22.1M |  5.2M  | x 0.24 |
|  accessor_set.rb   | 22.1M |  5.2M  | x 0.24 |
|    ivar_get.rb     | 22.0M |  5.2M  | x 0.24 |
|    ivar_set.rb     | 22.1M |  5.2M  | x 0.24 |
|   loop_times.rb    | 22.1M |  5.2M  | x 0.24 |
|    loop_for.rb     | 22.1M |  5.2M  | x 0.24 |
| loop_whileloop.rb  | 22.0M |  5.2M  | x 0.24 |
| so_concatenate.rb  | 73.4M | 63.8M  | x 0.87 |
| string_scan_str.rb | 26.8M |  6.7M  | x 0.25 |
| string_scan_re.rb  | 26.9M |  6.7M  | x 0.25 |
| fiber_allocate.rb  | 46.0M | 211.2M | x 4.59 |
|  fiber_switch.rb   | 22.1M |  5.2M  | x 0.23 |
|  so_mandelbrot.rb  | 22.3M |  6.0M  | x 0.27 |
| app_mandelbrot.rb  | 22.3M |  6.0M  | x 0.27 |
|    app_fibo.rb     | 22.1M |  5.2M  | x 0.24 |
|   app_aobench.rb   | 22.7M |  8.3M  | x 0.36 |
|    so_nbody.rb     | 22.1M |  5.6M  | x 0.25 |
|     collatz.rb     | 22.1M |  5.2M  | x 0.24 |
|     optcarrot      | 78.2M | 64.8M  | x 0.83 |
|  optcarrot --opt   | 85.8M | 802.9M | x 9.36 |
