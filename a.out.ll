; ModuleID = "a.out.ll"
target triple = "unknown-unknown-unknown"
target datalayout = ""

define i64 @"main"(i64 %".1", i8** %".2") 
{
entry0:
  %".4" = sitofp i64 8 to float
  %".5" = fmul float 0x4016000000000000, %".4"
  %".6" = sitofp i64 3 to float
  %".7" = fdiv float %".6", 0x4000000000000000
  %".8" = fadd float %".5", %".7"
  %".9" = getelementptr i8*, i8** %".2", i64 0
  %".10" = load i8*, i8** %".9"
  %".11" = getelementptr i8, i8* %".10", i64 2
  %".12" = load i8, i8* %".11"
  %".13" = fpext float %".8" to double
  %".14" = bitcast [11 x i8]* @"fstrifi" to i8*
  %".15" = call i32 (i8*, ...) @"printf"(i8* %".14", i64 %".1", double %".13", i8 %".12")
  ret i64 0
}

declare i32 @"printf"(i8* %".1", ...) 

@"fstrifi" = internal constant [11 x i8] c"%d %f %d \0a\00"