# ruruby benchmark results

## environment

Ruby version: 3.0.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 20.04.1 LTS

## execution time

|     benchmark      |      ruby      |     ruruby     |  rate  |
| :----------------: | :------------: | :------------: | :----: |
|  accessor_get.rb   | 0.58 ± 0.01 s  | 1.14 ± 0.01 s  | x 1.95 |
|  accessor_set.rb   | 0.42 ± 0.00 s  | 1.29 ± 0.00 s  | x 3.10 |
|    ivar_get.rb     | 1.73 ± 0.00 s  | 1.48 ± 0.00 s  | x 0.86 |
|    ivar_set.rb     | 1.14 ± 0.02 s  | 2.25 ± 0.01 s  | x 1.97 |
|   loop_times.rb    | 0.75 ± 0.03 s  | 0.44 ± 0.00 s  | x 0.58 |
|    loop_for.rb     | 0.82 ± 0.02 s  | 0.50 ± 0.00 s  | x 0.60 |
| loop_whileloop.rb  | 0.41 ± 0.01 s  | 0.55 ± 0.00 s  | x 1.35 |
| so_concatenate.rb  | 0.71 ± 0.01 s  | 0.67 ± 0.01 s  | x 0.94 |
| string_scan_str.rb | 1.15 ± 0.10 s  | 1.07 ± 0.03 s  | x 0.93 |
| string_scan_re.rb  | 1.57 ± 0.01 s  | 1.05 ± 0.01 s  | x 0.67 |
| fiber_allocate.rb  | 1.37 ± 0.04 s  | 0.72 ± 0.02 s  | x 0.52 |
|  fiber_switch.rb   | 0.78 ± 0.01 s  | 1.00 ± 0.01 s  | x 1.28 |
|  so_mandelbrot.rb  | 1.83 ± 0.01 s  | 2.67 ± 0.03 s  | x 1.46 |
| app_mandelbrot.rb  | 1.39 ± 0.00 s  | 1.32 ± 0.17 s  | x 0.95 |
|    app_fibo.rb     | 0.59 ± 0.05 s  | 1.36 ± 0.04 s  | x 2.31 |
|   app_aobench.rb   | 10.65 ± 0.34 s | 19.72 ± 0.25 s | x 1.85 |
|    so_nbody.rb     | 1.07 ± 0.06 s  | 2.55 ± 0.03 s  | x 2.38 |
|     collatz.rb     | 6.35 ± 0.08 s  | 8.34 ± 0.10 s  | x 1.31 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 40.21 ± 2.43 fps  | 13.67 ± 0.73 fps | x 2.94 |
| optcarrot --opt | 129.23 ± 0.60 fps | 55.54 ± 6.03 fps | x 2.33 |

## memory consumption

|     benchmark      | ruby  | ruruby |  rate  |
| :----------------: | :---: | :----: | :----: |
|  accessor_get.rb   | 22.1M |  5.0M  | x 0.23 |
|  accessor_set.rb   | 22.0M |  5.0M  | x 0.23 |
|    ivar_get.rb     | 22.0M |  5.0M  | x 0.23 |
|    ivar_set.rb     | 22.0M |  5.0M  | x 0.23 |
|   loop_times.rb    | 22.0M |  5.0M  | x 0.23 |
|    loop_for.rb     | 22.0M |  5.0M  | x 0.23 |
| loop_whileloop.rb  | 22.0M |  4.9M  | x 0.22 |
| so_concatenate.rb  | 71.2M | 63.7M  | x 0.89 |
| string_scan_str.rb | 27.1M |  6.6M  | x 0.24 |
| string_scan_re.rb  | 26.8M |  6.6M  | x 0.25 |
| fiber_allocate.rb  | 45.9M | 211.2M | x 4.60 |
|  fiber_switch.rb   | 22.1M |  5.0M  | x 0.23 |
|  so_mandelbrot.rb  | 22.1M |  5.8M  | x 0.26 |
| app_mandelbrot.rb  | 22.0M |  5.8M  | x 0.26 |
|    app_fibo.rb     | 22.0M |  5.0M  | x 0.23 |
|   app_aobench.rb   | 22.6M |  8.2M  | x 0.36 |
|    so_nbody.rb     | 22.0M |  5.4M  | x 0.24 |
|     collatz.rb     | 22.0M |  5.0M  | x 0.23 |
|     optcarrot      | 77.5M | 64.2M  | x 0.83 |
|  optcarrot --opt   | 88.6M | 811.6M | x 9.16 |
