using System.Globalization;

internal static class Program
{
    private const int Rounds = 2;
    private const int IntegerIterations = 750_000;
    private const int FloatIterations = 150_000;
    private const int ListSize = 100_000;

    private static long IntegerWork()
    {
        long state = 1;
        long checksum = 0;
        for (var i = 0; i < IntegerIterations; i++)
        {
            state = (state * 1_664_525 + 1_013_904_223 + i) % 2_147_483_647;
            checksum = (checksum + state) % 9_223_372_036_854_775_000;
        }
        return checksum;
    }

    private static double FloatingWork()
    {
        var checksum = 0.0;
        for (var i = 0; i < FloatIterations; i++)
        {
            var x = (i + 1) * 0.00001;
            checksum += Math.Sin(x) * Math.Cos(x) + Math.Sqrt(x + 1.0) + Math.Log(x + 1.0);
        }
        return checksum;
    }

    private static long ListWork()
    {
        var values = new List<long>();
        long state = 7;
        for (var i = 0; i < ListSize; i++)
        {
            state = (state * 48_271 + i) % 2_147_483_647;
            values.Add(state);
        }
        values.Sort();
        var middle = ListSize / 2;
        return values[0] + values[middle] + values[ListSize - 1] + values.Count;
    }

    private static void Main()
    {
        long integerChecksum = 0;
        var floatingChecksum = 0.0;
        long listChecksum = 0;

        for (var roundIndex = 0; roundIndex < Rounds; roundIndex++)
        {
            integerChecksum += IntegerWork() + roundIndex;
            floatingChecksum += FloatingWork();
            listChecksum += ListWork();
        }

        Console.WriteLine($"integer={integerChecksum}");
        Console.WriteLine($"float={floatingChecksum.ToString("F6", CultureInfo.InvariantCulture)}");
        Console.WriteLine($"list={listChecksum}");
    }
}
