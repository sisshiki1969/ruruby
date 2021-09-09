# ruruby benchmark results

## environment

2021-09-07 02:44:06 +0900

Ruby version: 3.0.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 20.04.1 LTS  
branch: master

## execution time

|     benchmark      |     ruby      |     ruruby     |  rate  |
| :----------------: | :-----------: | :------------: | :----: |
|   simple_call.rb   | 0.20 ± 0.01 s | 0.43 ± 0.01 s  | x 2.22 |
|  accessor_get.rb   | 0.55 ± 0.01 s | 0.82 ± 0.03 s  | x 1.49 |
|  accessor_set.rb   | 0.40 ± 0.01 s | 0.95 ± 0.02 s  | x 2.41 |
|    ivar_get.rb     | 0.85 ± 0.01 s | 0.66 ± 0.01 s  | x 0.78 |
|    ivar_set.rb     | 0.57 ± 0.01 s | 1.02 ± 0.06 s  | x 1.78 |
|   loop_times.rb    | 0.74 ± 0.04 s | 0.75 ± 0.14 s  | x 1.02 |
|    loop_for.rb     | 0.80 ± 0.05 s | 0.88 ± 0.01 s  | x 1.10 |
| loop_whileloop.rb  | 0.39 ± 0.01 s | 0.58 ± 0.01 s  | x 1.48 |
| so_concatenate.rb  | 0.67 ± 0.00 s | 0.61 ± 0.01 s  | x 0.91 |
| string_scan_str.rb | 1.06 ± 0.01 s | 1.02 ± 0.01 s  | x 0.96 |
| string_scan_re.rb  | 1.51 ± 0.01 s | 0.99 ± 0.01 s  | x 0.66 |
| fiber_allocate.rb  | 1.41 ± 0.22 s | 0.82 ± 0.03 s  | x 0.58 |
|  fiber_switch.rb   | 0.75 ± 0.01 s | 0.91 ± 0.01 s  | x 1.22 |
|  so_mandelbrot.rb  | 1.66 ± 0.04 s | 2.13 ± 0.03 s  | x 1.28 |
| app_mandelbrot.rb  | 1.30 ± 0.03 s | 1.22 ± 0.01 s  | x 0.94 |
|    app_fibo.rb     | 0.50 ± 0.01 s | 1.02 ± 0.01 s  | x 2.04 |
|   app_aobench.rb   | 9.22 ± 0.29 s | 15.71 ± 0.14 s | x 1.70 |
|    so_nbody.rb     | 0.99 ± 0.01 s | 1.68 ± 0.06 s  | x 1.70 |
|     collatz.rb     | 6.07 ± 0.06 s | 7.65 ± 0.07 s  | x 1.26 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 42.81 ± 1.10 fps  | 18.40 ± 0.37 fps | x 2.33 |
| optcarrot --opt | 140.05 ± 1.40 fps | 70.11 ± 2.28 fps | x 2.00 |

## memory consumption

|     benchmark      | ruby  | ruruby |  rate  |
| :----------------: | :---: | :----: | :----: |
|   simple_call.rb   | 22.0M |  5.6M  | x 0.25 |
|  accessor_get.rb   | 22.0M |  5.5M  | x 0.25 |
|  accessor_set.rb   | 22.0M |  5.6M  | x 0.25 |
|    ivar_get.rb     | 22.0M |  5.6M  | x 0.25 |
|    ivar_set.rb     | 22.0M |  5.6M  | x 0.25 |
|   loop_times.rb    | 22.0M |  5.6M  | x 0.26 |
|    loop_for.rb     | 22.0M |  5.5M  | x 0.25 |
| loop_whileloop.rb  | 22.0M |  5.6M  | x 0.25 |
| so_concatenate.rb  | 74.8M | 20.7M  | x 0.28 |
| string_scan_str.rb | 26.8M |  7.3M  | x 0.27 |
| string_scan_re.rb  | 26.7M |  7.2M  | x 0.27 |
| fiber_allocate.rb  | 49.2M | 320.1M | x 6.50 |
|  fiber_switch.rb   | 22.0M |  5.5M  | x 0.25 |
|  so_mandelbrot.rb  | 22.2M |  6.4M  | x 0.29 |
| app_mandelbrot.rb  | 22.2M |  6.3M  | x 0.29 |
|    app_fibo.rb     | 22.0M |  5.6M  | x 0.25 |
|   app_aobench.rb   | 22.7M |  7.0M  | x 0.31 |
|    so_nbody.rb     | 22.1M |  5.7M  | x 0.26 |
|     collatz.rb     | 22.0M |  5.6M  | x 0.26 |
|     optcarrot      | 78.2M | 64.4M  | x 0.82 |
|  optcarrot --opt   | 88.8M | 81.5M  | x 0.92 |
