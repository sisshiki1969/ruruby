# ruruby benchmark results

## environment

2021-08-22 13:25:21 +0900

Ruby version: 3.0.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 20.04.1 LTS  
branch: master

## execution time

|     benchmark      |     ruby      |     ruruby     |  rate  |
| :----------------: | :-----------: | :------------: | :----: |
|   simple_call.rb   | 0.19 ± 0.00 s | 0.43 ± 0.01 s  | x 2.21 |
|  accessor_get.rb   | 0.58 ± 0.01 s | 0.88 ± 0.02 s  | x 1.52 |
|  accessor_set.rb   | 0.41 ± 0.00 s | 1.07 ± 0.04 s  | x 2.61 |
|    ivar_get.rb     | 0.89 ± 0.01 s | 0.77 ± 0.02 s  | x 0.86 |
|    ivar_set.rb     | 0.59 ± 0.00 s | 1.05 ± 0.03 s  | x 1.78 |
|   loop_times.rb    | 0.75 ± 0.02 s | 0.75 ± 0.04 s  | x 1.00 |
|    loop_for.rb     | 0.84 ± 0.05 s | 0.96 ± 0.06 s  | x 1.14 |
| loop_whileloop.rb  | 0.41 ± 0.00 s | 0.65 ± 0.01 s  | x 1.59 |
| so_concatenate.rb  | 0.70 ± 0.01 s | 0.64 ± 0.01 s  | x 0.90 |
| string_scan_str.rb | 1.13 ± 0.02 s | 0.99 ± 0.03 s  | x 0.87 |
| string_scan_re.rb  | 1.58 ± 0.01 s | 0.94 ± 0.01 s  | x 0.60 |
| fiber_allocate.rb  | 1.47 ± 0.07 s | 1.04 ± 0.06 s  | x 0.71 |
|  fiber_switch.rb   | 0.79 ± 0.02 s | 0.96 ± 0.02 s  | x 1.22 |
|  so_mandelbrot.rb  | 1.74 ± 0.03 s | 2.37 ± 0.06 s  | x 1.36 |
| app_mandelbrot.rb  | 1.38 ± 0.06 s | 1.35 ± 0.07 s  | x 0.98 |
|    app_fibo.rb     | 0.52 ± 0.02 s | 1.11 ± 0.02 s  | x 2.13 |
|   app_aobench.rb   | 9.94 ± 0.14 s | 17.64 ± 0.38 s | x 1.77 |
|    so_nbody.rb     | 1.04 ± 0.02 s | 2.06 ± 0.05 s  | x 1.98 |
|     collatz.rb     | 6.35 ± 0.05 s | 9.16 ± 0.10 s  | x 1.44 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 40.50 ± 0.85 fps  | 16.43 ± 1.05 fps | x 2.47 |
| optcarrot --opt | 128.86 ± 5.27 fps | 61.90 ± 4.29 fps | x 2.08 |

## memory consumption

|     benchmark      | ruby  | ruruby |  rate   |
| :----------------: | :---: | :----: | :-----: |
|   simple_call.rb   | 22.1M |  5.6M  | x 0.25  |
|  accessor_get.rb   | 22.0M |  5.6M  | x 0.25  |
|  accessor_set.rb   | 22.0M |  5.6M  | x 0.25  |
|    ivar_get.rb     | 22.1M |  5.7M  | x 0.26  |
|    ivar_set.rb     | 22.0M |  5.6M  | x 0.25  |
|   loop_times.rb    | 22.1M |  5.6M  | x 0.25  |
|    loop_for.rb     | 22.1M |  5.6M  | x 0.25  |
| loop_whileloop.rb  | 22.0M |  5.6M  | x 0.25  |
| so_concatenate.rb  | 74.9M | 64.3M  | x 0.86  |
| string_scan_str.rb | 26.8M |  7.2M  | x 0.27  |
| string_scan_re.rb  | 26.8M |  7.2M  | x 0.27  |
| fiber_allocate.rb  | 55.9M | 409.8M | x 7.33  |
|  fiber_switch.rb   | 22.1M |  5.7M  | x 0.26  |
|  so_mandelbrot.rb  | 22.2M |  6.5M  | x 0.29  |
| app_mandelbrot.rb  | 22.3M |  6.4M  | x 0.29  |
|    app_fibo.rb     | 22.1M |  5.6M  | x 0.26  |
|   app_aobench.rb   | 22.7M |  8.7M  | x 0.38  |
|    so_nbody.rb     | 22.0M |  5.7M  | x 0.26  |
|     collatz.rb     | 22.1M |  5.7M  | x 0.26  |
|     optcarrot      | 77.1M | 64.7M  | x 0.84  |
|  optcarrot --opt   | 91.6M | 965.3M | x 10.54 |
