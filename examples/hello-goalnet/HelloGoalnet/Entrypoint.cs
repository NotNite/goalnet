using System.Runtime.InteropServices;

namespace HelloGoalnet;

public class Entrypoint {
    public delegate void MainDelegate(nint unloadPtr);
    public delegate void UnloadDelegate();

    private static UnloadDelegate UnloadFunc = null!;

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int MessageBox(nint hWnd, string text, string caption, uint type);

    public static void Main(nint unloadPtr) {
        UnloadFunc = Marshal.GetDelegateForFunctionPointer<UnloadDelegate>(unloadPtr);

        // do whatever you want
        MessageBox(nint.Zero, "Hello from the Entrypoint class!", "hello-goalnet", 0);

        // unload immediately for demonstration - you can persist as long as you want
        // make sure to cleanup everything you do before calling unload
        UnloadFunc();
    }
}
