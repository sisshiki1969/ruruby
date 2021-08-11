# ruruby benchmark results

(using ahash)

## environment

2021-08-11 12:23:34 +0900

Ruby version: 3.0.1  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 20.04.2 LTS  
branch: master

## execution time

|     benchmark      |     ruby      |     ruruby     |  rate  |
| :----------------: | :-----------: | :------------: | :----: |
|   simple_call.rb   | 0.20 ± 0.01 s | 0.47 ± 0.01 s  | x 2.35 |
|  accessor_get.rb   | 0.53 ± 0.02 s | 1.04 ± 0.03 s  | x 1.97 |
|  accessor_set.rb   | 0.39 ± 0.01 s | 1.26 ± 0.03 s  | x 3.24 |
|    ivar_get.rb     | 0.91 ± 0.00 s | 0.74 ± 0.01 s  | x 0.82 |
|    ivar_set.rb     | 0.65 ± 0.01 s | 1.18 ± 0.10 s  | x 1.82 |
|   loop_times.rb    | 0.77 ± 0.01 s | 0.76 ± 0.01 s  | x 0.98 |
|    loop_for.rb     | 0.88 ± 0.05 s | 0.94 ± 0.02 s  | x 1.07 |
| loop_whileloop.rb  | 0.42 ± 0.02 s | 0.64 ± 0.04 s  | x 1.52 |
| so_concatenate.rb  | 0.70 ± 0.03 s | 0.86 ± 0.01 s  | x 1.22 |
| string_scan_str.rb | 1.11 ± 0.01 s | 0.97 ± 0.00 s  | x 0.87 |
| string_scan_re.rb  | 1.59 ± 0.01 s | 0.96 ± 0.01 s  | x 0.60 |
| fiber_allocate.rb  | 1.73 ± 0.27 s | 0.97 ± 0.01 s  | x 0.56 |
|  fiber_switch.rb   | 0.92 ± 0.01 s | 1.20 ± 0.03 s  | x 1.31 |
|  so_mandelbrot.rb  | 1.70 ± 0.02 s | 1.99 ± 0.04 s  | x 1.17 |
| app_mandelbrot.rb  | 1.27 ± 0.01 s | 1.36 ± 0.09 s  | x 1.07 |
|    app_fibo.rb     | 0.50 ± 0.01 s | 1.20 ± 0.02 s  | x 2.39 |
|   app_aobench.rb   | 9.69 ± 0.51 s | 20.60 ± 0.73 s | x 2.13 |
|    so_nbody.rb     | 1.07 ± 0.03 s | 2.10 ± 0.05 s  | x 1.95 |
|     collatz.rb     | 7.05 ± 0.17 s | 9.97 ± 0.18 s  | x 1.41 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 39.05 ± 1.52 fps  | 16.59 ± 0.22 fps | x 2.35 |
| optcarrot --opt | 128.89 ± 2.46 fps | 61.92 ± 5.11 fps | x 2.08 |

## memory consumption

|     benchmark      | ruby  | ruruby |  rate  |
| :----------------: | :---: | :----: | :----: |
|   simple_call.rb   | 22.0M |  5.3M  | x 0.24 |
|  accessor_get.rb   | 22.0M |  5.2M  | x 0.24 |
|  accessor_set.rb   | 22.0M |  5.3M  | x 0.24 |
|    ivar_get.rb     | 22.0M |  5.2M  | x 0.24 |
|    ivar_set.rb     | 22.0M |  5.3M  | x 0.24 |
|   loop_times.rb    | 22.0M |  5.2M  | x 0.24 |
|    loop_for.rb     | 22.0M |  5.2M  | x 0.24 |
| loop_whileloop.rb  | 22.0M |  5.2M  | x 0.24 |
| so_concatenate.rb  | 70.6M | 64.0M  | x 0.91 |
| string_scan_str.rb | 27.0M |  6.8M  | x 0.25 |
| string_scan_re.rb  | 27.0M |  6.8M  | x 0.25 |
| fiber_allocate.rb  | 70.1M | 423.9M | x 6.05 |
|  fiber_switch.rb   | 22.0M |  5.2M  | x 0.24 |
|  so_mandelbrot.rb  | 22.4M |  6.1M  | x 0.27 |
| app_mandelbrot.rb  | 22.3M |  6.0M  | x 0.27 |
|    app_fibo.rb     | 22.0M |  5.2M  | x 0.24 |
|   app_aobench.rb   | 22.8M |  8.7M  | x 0.38 |
|    so_nbody.rb     | 22.0M |  5.6M  | x 0.26 |
|     collatz.rb     | 22.0M |  5.3M  | x 0.24 |
|     optcarrot      | 77.0M | 64.7M  | x 0.84 |
|  optcarrot --opt   | 94.1M | 816.2M | x 8.67 |
