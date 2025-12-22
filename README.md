Project based on [PSOC_RUST](https://github.com/9names/PSOC_Rust/tree/master). PSoC 5LP
# PSoC 42xx CY8C4245AXI-485
Using PSoC4 Protoyping kit CY8CKIt-049-42XX I have created a project with PSOC Creator IDE and integrated it with an embedded rust application that uses all the HAL and the auto-generated code from the IDE

This project shows a very interesting way to modernize existing C projects reusing existing and tested functionality

Since this MCU is old enough it is very unlikely that it will get much traction for fully embedded Rust frameworks, HAL, PAC, to be used with Embassy, RTIC, etc.

Although limited this is a simple wayto start exploting Rust NO_STD for embedded, since you can reuse already known functions.

There is also an option to expose rust functions to be used in the C application.

I have written a blog post in Medium explaining some of the process.

the application is not finished yet but the process works.


