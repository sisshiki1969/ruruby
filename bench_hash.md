# ruruby benchmark results

(using std::collections::HashMap)

## environment

2021-08-11 10:15:38 +0900

Ruby version: 3.0.1  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 20.04.2 LTS  
branch: master

## execution time

|     benchmark      |     ruby      |     ruruby     |  rate  |
| :----------------: | :-----------: | :------------: | :----: |
|   simple_call.rb   | 0.19 ± 0.00 s | 0.46 ± 0.01 s  | x 2.39 |
|  accessor_get.rb   | 0.51 ± 0.00 s | 1.04 ± 0.01 s  | x 2.02 |
|  accessor_set.rb   | 0.38 ± 0.00 s | 1.57 ± 0.02 s  | x 4.09 |
|    ivar_get.rb     | 0.91 ± 0.00 s | 0.82 ± 0.01 s  | x 0.89 |
|    ivar_set.rb     | 0.65 ± 0.01 s | 2.00 ± 0.00 s  | x 3.08 |
|   loop_times.rb    | 0.77 ± 0.02 s | 0.75 ± 0.02 s  | x 0.97 |
|    loop_for.rb     | 0.86 ± 0.04 s | 0.94 ± 0.02 s  | x 1.10 |
| loop_whileloop.rb  | 0.40 ± 0.00 s | 0.59 ± 0.00 s  | x 1.47 |
| so_concatenate.rb  | 0.69 ± 0.01 s | 1.00 ± 0.00 s  | x 1.45 |
| string_scan_str.rb | 1.10 ± 0.01 s | 0.98 ± 0.01 s  | x 0.89 |
| string_scan_re.rb  | 1.58 ± 0.01 s | 0.97 ± 0.02 s  | x 0.61 |
| fiber_allocate.rb  | 1.74 ± 0.26 s | 0.97 ± 0.00 s  | x 0.56 |
|  fiber_switch.rb   | 0.90 ± 0.00 s | 1.20 ± 0.01 s  | x 1.33 |
|  so_mandelbrot.rb  | 1.71 ± 0.01 s | 2.05 ± 0.02 s  | x 1.20 |
| app_mandelbrot.rb  | 1.25 ± 0.00 s | 1.43 ± 0.01 s  | x 1.14 |
|    app_fibo.rb     | 0.51 ± 0.01 s | 1.15 ± 0.03 s  | x 2.27 |
|   app_aobench.rb   | 9.55 ± 0.06 s | 22.38 ± 0.13 s | x 2.34 |
|    so_nbody.rb     | 1.06 ± 0.01 s | 2.45 ± 0.01 s  | x 2.32 |
|     collatz.rb     | 6.73 ± 0.07 s | 8.49 ± 0.07 s  | x 1.26 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 41.02 ± 0.42 fps  | 12.02 ± 0.16 fps | x 3.41 |
| optcarrot --opt | 132.91 ± 0.66 fps | 58.37 ± 0.55 fps | x 2.28 |

## memory consumption

|     benchmark      | ruby  | ruruby |  rate  |
| :----------------: | :---: | :----: | :----: |
|   simple_call.rb   | 22.0M |  5.2M  | x 0.24 |
|  accessor_get.rb   | 22.0M |  5.3M  | x 0.24 |
|  accessor_set.rb   | 22.0M |  5.3M  | x 0.24 |
|    ivar_get.rb     | 22.0M |  5.3M  | x 0.24 |
|    ivar_set.rb     | 22.0M |  5.3M  | x 0.24 |
|   loop_times.rb    | 22.0M |  5.3M  | x 0.24 |
|    loop_for.rb     | 22.0M |  5.3M  | x 0.24 |
| loop_whileloop.rb  | 21.9M |  5.2M  | x 0.24 |
| so_concatenate.rb  | 69.2M | 63.9M  | x 0.92 |
| string_scan_str.rb | 26.9M |  6.8M  | x 0.25 |
| string_scan_re.rb  | 26.9M |  6.8M  | x 0.25 |
| fiber_allocate.rb  | 70.1M | 424.0M | x 6.05 |
|  fiber_switch.rb   | 22.0M |  5.3M  | x 0.24 |
|  so_mandelbrot.rb  | 22.3M |  6.1M  | x 0.27 |
| app_mandelbrot.rb  | 22.4M |  6.1M  | x 0.27 |
|    app_fibo.rb     | 22.0M |  5.3M  | x 0.24 |
|   app_aobench.rb   | 22.8M |  8.5M  | x 0.37 |
|    so_nbody.rb     | 22.0M |  5.7M  | x 0.26 |
|     collatz.rb     | 22.0M |  5.3M  | x 0.24 |
|     optcarrot      | 77.0M | 64.7M  | x 0.84 |
|  optcarrot --opt   | 94.1M | 816.2M | x 8.67 |
