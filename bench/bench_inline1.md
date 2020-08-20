# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 20.04 LTS

## execution time

|     benchmark     |      ruby      |     ruruby     |  rate  |
| :---------------: | :------------: | :------------: | :----: |
|     block.rb      | 0.25 ± 0.01 s  | 0.48 ± 0.03 s  | x 1.91 |
|    for_loop.rb    | 0.32 ± 0.01 s  | 0.41 ± 0.00 s  | x 1.27 |
| so_mandelbrot.rb  | 2.07 ± 0.01 s  | 2.16 ± 0.10 s  | x 1.05 |
| app_mandelbrot.rb | 1.16 ± 0.01 s  | 1.05 ± 0.01 s  | x 0.91 |
|    app_fibo.rb    | 0.53 ± 0.00 s  | 1.68 ± 0.01 s  | x 3.15 |
|  app_aobench.rb   | 10.57 ± 0.13 s | 20.42 ± 0.17 s | x 1.93 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 43.73 ± 0.51 fps  | 13.61 ± 0.10 fps | x 3.21 |
| optcarrot --opt | 132.99 ± 3.87 fps | 53.69 ± 2.15 fps | x 2.48 |

## memory consumption

|     benchmark     | ruby  | ruruby  |  rate   |
| :---------------: | :---: | :-----: | :-----: |
|     block.rb      | 21.9M |  4.6M   | x 0.21  |
|    for_loop.rb    | 21.9M |  4.6M   | x 0.21  |
| so_mandelbrot.rb  | 22.3M |  5.9M   | x 0.26  |
| app_mandelbrot.rb | 22.3M |  5.5M   | x 0.25  |
|    app_fibo.rb    | 21.9M |  4.7M   | x 0.22  |
|  app_aobench.rb   | 22.6M |  8.0M   | x 0.35  |
|     optcarrot     | 76.8M |  64.3M  | x 0.84  |
|  optcarrot --opt  | 94.0M | 1027.2M | x 10.93 |
